# Security policy

## Supported scope

Looma is a pre-release prototype with no stable releases or support SLA. Security
fixes currently target the latest source on the default branch. This policy does
not claim the project or its dependencies have passed a security audit.

## Report privately

After the public repository is created, use its **Security → Report a
vulnerability** form. Do not include exploit details, credentials, personal data,
or private prompts in a public issue.

Private vulnerability reporting must be enabled by the maintainer before the
repository is opened publicly; see [the publication checklist](docs/PUBLICATION_CHECKLIST.md).
Until that channel is available, do not send sensitive details through public
issues. For an accidentally exposed credential, revoke/rotate it immediately
through the provider without waiting for a maintainer response.

Include affected versions, a minimal reproduction using synthetic data, expected
impact, and relevant redacted logs. No response-time guarantee is offered yet.

## Trust boundaries

Looma launches the locally installed `codex` executable using local account
authentication. Text is sent to that tool and its service. Image generation uses
a writable directory and a model-driven subprocess. Sandbox flags reduce access;
they are not a guarantee of isolation. Do not use sensitive prompts or run
untrusted changes with credentials. See [README.md](README.md) for data handling,
timeouts, and the build-time image output path.
