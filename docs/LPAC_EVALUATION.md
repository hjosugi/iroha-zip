# LPAC evaluation

Updated: 2026-08-09

This document records the SAFE-006 prototype boundary. LPAC remains opt-in until the real-Windows archive and denial matrix is complete.

## What changes in LPAC mode

Both modes create an ephemeral AppContainer profile, pass a `SECURITY_CAPABILITIES` structure with zero capabilities, inherit only stdin/stdout/stderr, and attach the backend to the same one-process Job Object.

LPAC additionally sets `PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY` to `PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT`. Microsoft describes LPAC as an AppContainer that must explicitly declare access to resources which a regular AppContainer can reach, and its current launch example uses this exact opt-out attribute. Microsoft also explains that many Windows files, registry keys, and COM resources grant access to `ALL APPLICATION PACKAGES`; LPAC does not receive that implicit access.

Primary references:

- [Launch an AppContainer or LPAC](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer)
- [`TOKEN_INFORMATION_CLASS` and `TokenIsLessPrivilegedAppContainer`](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ne-winnt-token_information_class)
- [`ALL APPLICATION PACKAGES` and COM restrictions](https://learn.microsoft.com/en-us/windows/win32/com/donotaddallapplicationpackagestorestrictions)
- [`UpdateProcThreadAttribute`](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)

## Configuration and fail-closed behavior

The settings screen exposes two values, serialized under `[sandbox]`:

```toml
isolation = "appcontainer" # default compatibility mode
isolation = "lpac"         # experimental stricter mode
```

When LPAC is selected, iroha-zip does not retry process creation as a regular AppContainer. It opens the created process token and requires both `TokenIsAppContainer != 0` and `TokenIsLessPrivilegedAppContainer != 0`. Missing API support, rejected attributes, backend loader failure, and token downgrade are errors. This deliberately uses runtime evidence instead of assuming support from an OS version string.

The existing `--allow-unsandboxed` switch is separate. It permits an unsandboxed diagnostic fallback only when the user explicitly supplies that switch for that individual command; it is never read from persistent configuration.

## Capability policy

The prototype grants no capabilities. In particular, it does not derive or add `internetClient`, `privateNetworkClientServer`, `registryRead`, `lpacCom`, or broad application-experience capabilities. A compatibility failure is evidence to investigate; it is not permission to widen the default capability set.

Any proposed capability must include:

1. the exact failing backend operation and Windows build;
2. an official definition for the capability;
3. a before/after denial matrix;
4. proof that no narrower file ACL or implementation change works;
5. a separate security review before it can become a default.

## Validation matrix still required

The settings-screen diagnostic runs `bsdtar --version` in the selected mode and therefore exercises process creation, DLL loading, standard-handle inheritance, Job Object assignment, and token verification. It is not sufficient archive coverage.

SAFE-006 remains open until disposable Windows 10 and Windows 11 x64 workers record, for both normal AppContainer and LPAC:

- token AppContainer/LPAC flags and zero capability count;
- ZIP, 7z, RAR, LZH, TAR.GZ, and `.Z` extraction;
- ZIP, 7z, TAR, and TAR.GZ creation plus re-extraction comparison;
- access denial for user documents, representative registry keys, COM activation, loopback, LAN, and Internet;
- access allowed only to the ephemeral profile, inherited standard handles, backend bundle, input, and output;
- timeout, crash, loader failure, and cleanup behavior with no profile or temporary tree left behind.

No current result should be interpreted as an independent Windows sandbox audit.
