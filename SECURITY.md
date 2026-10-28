# Security

Do not post passwords, access tokens, signing keys, private network build identities, saves, or unreviewed logs in a public issue.

If you believe a credential was exposed, revoke it with its provider and replace it through a private channel. For a vulnerability in the updater or release process, contact Smokey privately through a channel you already trust; do not include exploit details or credentials in a public issue while a fix is being coordinated.

The updater verifies signed mod manifests and file hashes. It downloads an official fallback only when the official release metadata and archive hash can be verified. These checks do not guarantee the security of your PC or of every future dependency.
