use std::fmt;

const MAX_COMMAND_LINE_UNITS: usize = 32_767;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommandLineError {
    EmptyProgram,
    ProgramContainsQuote,
    InteriorNul,
    TooLong,
}

impl fmt::Display for CommandLineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyProgram => "program path is empty",
            Self::ProgramContainsQuote => "program path contains a quotation mark",
            Self::InteriorNul => "program path or argument contains NUL",
            Self::TooLong => "command line exceeds 32,767 UTF-16 code units",
        };
        formatter.write_str(message)
    }
}

pub(crate) fn encode(
    program: &[u16],
    arguments: &[Vec<u16>],
) -> Result<Vec<u16>, CommandLineError> {
    if program.is_empty() {
        return Err(CommandLineError::EmptyProgram);
    }
    if program.contains(&u16::from(b'"')) {
        return Err(CommandLineError::ProgramContainsQuote);
    }

    let mut output = Vec::new();
    append_quoted_argument(&mut output, program)?;
    for argument in arguments {
        push(&mut output, u16::from(b' '))?;
        append_quoted_argument(&mut output, argument)?;
    }
    push(&mut output, 0)?;
    Ok(output)
}

#[cfg(any(test, feature = "fuzzing"))]
pub(crate) fn quote(argument: &[u16]) -> Result<Vec<u16>, CommandLineError> {
    let mut output = Vec::new();
    append_quoted_argument(&mut output, argument)?;
    Ok(output)
}

fn append_quoted_argument(output: &mut Vec<u16>, argument: &[u16]) -> Result<(), CommandLineError> {
    if argument.contains(&0) {
        return Err(CommandLineError::InteriorNul);
    }

    let needs_quotes = argument.is_empty()
        || argument
            .iter()
            .any(|unit| matches!(*unit, 0x09 | 0x0a | 0x0b | 0x0c | 0x0d | 0x20 | 0x22));
    if !needs_quotes {
        extend(output, argument.iter().copied())?;
        return Ok(());
    }

    push(output, u16::from(b'"'))?;
    let mut backslashes = 0usize;
    for &unit in argument {
        if unit == u16::from(b'\\') {
            backslashes = backslashes.saturating_add(1);
            continue;
        }
        if unit == u16::from(b'"') {
            repeat(
                output,
                u16::from(b'\\'),
                backslashes.saturating_mul(2).saturating_add(1),
            )?;
            push(output, unit)?;
            backslashes = 0;
            continue;
        }
        repeat(output, u16::from(b'\\'), backslashes)?;
        backslashes = 0;
        push(output, unit)?;
    }
    repeat(output, u16::from(b'\\'), backslashes.saturating_mul(2))?;
    push(output, u16::from(b'"'))?;
    Ok(())
}

fn push(output: &mut Vec<u16>, unit: u16) -> Result<(), CommandLineError> {
    if output.len() >= MAX_COMMAND_LINE_UNITS {
        return Err(CommandLineError::TooLong);
    }
    output.push(unit);
    Ok(())
}

fn extend(
    output: &mut Vec<u16>,
    units: impl IntoIterator<Item = u16>,
) -> Result<(), CommandLineError> {
    for unit in units {
        push(output, unit)?;
    }
    Ok(())
}

fn repeat(output: &mut Vec<u16>, unit: u16, count: usize) -> Result<(), CommandLineError> {
    if count > MAX_COMMAND_LINE_UNITS.saturating_sub(output.len()) {
        return Err(CommandLineError::TooLong);
    }
    output.extend(std::iter::repeat_n(unit, count));
    Ok(())
}

#[cfg(any(test, feature = "fuzzing"))]
pub(crate) fn decode_single(encoded: &[u16]) -> Option<Vec<u16>> {
    let mut output = Vec::new();
    let mut index = 0usize;
    let mut quoted = false;

    while index < encoded.len() {
        if !quoted && matches!(encoded[index], 0x09 | 0x20) {
            return None;
        }

        let mut backslashes = 0usize;
        while index < encoded.len() && encoded[index] == u16::from(b'\\') {
            backslashes += 1;
            index += 1;
        }
        if index < encoded.len() && encoded[index] == u16::from(b'"') {
            output.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes / 2));
            if backslashes.is_multiple_of(2) {
                quoted = !quoted;
            } else {
                output.push(u16::from(b'"'));
            }
            index += 1;
        } else {
            output.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes));
            if index < encoded.len() {
                output.push(encoded[index]);
                index += 1;
            }
        }
    }

    (!quoted).then_some(output)
}

#[cfg(test)]
mod tests {
    use super::{CommandLineError, decode_single, encode, quote};

    #[test]
    fn quoted_arguments_round_trip() {
        for argument in [
            &[][..],
            &[b'a'.into(), b'b'.into()],
            &[b'a'.into(), b' '.into(), b'b'.into()],
            &[b'\\'.into()],
            &[b'\\'.into(), b'"'.into()],
            &[b'a'.into(), b'"'.into(), b'b'.into(), b'\\'.into()],
            &[0x3042, 0xd83d, 0xde00],
        ] {
            let encoded = quote(argument).unwrap();
            assert_eq!(decode_single(&encoded).as_deref(), Some(argument));
        }
    }

    #[test]
    fn quoting_round_trips_a_small_exhaustive_alphabet() {
        let alphabet = [
            u16::from(b'a'),
            u16::from(b' '),
            u16::from(b'\t'),
            u16::from(b'"'),
            u16::from(b'\\'),
        ];
        for length in 0..=4u32 {
            let cases = alphabet.len().pow(length);
            for mut case in 0..cases {
                let mut argument = Vec::with_capacity(length as usize);
                for _ in 0..length {
                    argument.push(alphabet[case % alphabet.len()]);
                    case /= alphabet.len();
                }
                let encoded = quote(&argument).unwrap();
                assert_eq!(decode_single(&encoded), Some(argument));
            }
        }
    }

    #[test]
    fn command_line_has_one_terminal_nul_and_a_bounded_length() {
        let encoded = encode(
            &[b'C'.into(), b':'.into(), b'\\'.into(), b'a'.into()],
            &[vec![b'x'.into(), b' '.into(), b'y'.into()]],
        )
        .unwrap();
        assert_eq!(encoded.last(), Some(&0));
        assert!(!encoded[..encoded.len() - 1].contains(&0));
        assert!(encoded.len() <= 32_767);
    }

    #[test]
    fn rejects_ambiguous_or_unrepresentable_input() {
        assert_eq!(encode(&[], &[]), Err(CommandLineError::EmptyProgram));
        assert_eq!(
            encode(&[b'"'.into()], &[]),
            Err(CommandLineError::ProgramContainsQuote)
        );
        assert_eq!(quote(&[0]), Err(CommandLineError::InteriorNul));
        assert_eq!(
            quote(&vec![u16::from(b' '); 32_767]),
            Err(CommandLineError::TooLong)
        );

        let largest_argument = vec![u16::from(b'a'); 32_764];
        assert_eq!(
            encode(&[u16::from(b'p')], &[largest_argument])
                .unwrap()
                .len(),
            32_767
        );
        assert_eq!(
            encode(&[u16::from(b'p')], &[vec![u16::from(b'a'); 32_765]]),
            Err(CommandLineError::TooLong)
        );
    }
}
