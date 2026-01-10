# Security Policy

## Supported Versions

Am currently working towards an initial "formal" release version at 0.1.0.  Support is negotiable, but the default is through the GitHub issues mechanism.

## Reporting a Vulnerability

Please post an issue.

## Security Considerations

**goa executes arbitrary commands** based on repository content. This is by design, but creates security risks that users should understand before deployment.

### Known Risks

#### 1. Arbitrary Code Execution
- goa executes commands from the `-c` flag or `.goa` file with the same privileges as the goa process
- A compromised repository can execute malicious code on your system
- **Mitigation**: Only monitor trusted repositories; use the `--timeout` flag to limit runaway commands

#### 2. Running as Root
- **Never run goa as root** unless absolutely necessary
- Commands executed by goa inherit root privileges, allowing full system compromise
- **Mitigation**: Run goa as a dedicated unprivileged user with minimal permissions

#### 3. Token Exposure
- Access tokens passed via `-t`/`--token` may appear in process listings (`ps aux`)
- Tokens are embedded in the cloned repository's remote URL
- **Mitigation**: Use environment variables or credential helpers when possible; restrict access to systems running goa

#### 4. Command Injection via Commit Metadata
- Environment variables (`GOA_LAST_COMMIT_AUTHOR`, `GOA_LAST_COMMIT_MESSAGE`, etc.) contain user-controlled data
- If your command uses these variables unsafely, attackers could inject malicious commands
- **Mitigation**: Always quote variables in shell commands; validate/sanitize input in your scripts

#### 5. Denial of Service
- Without `--timeout`, a malicious `.goa` file could run indefinitely, consuming resources
- Rapid commits could trigger many executions
- **Mitigation**: Use `--timeout` flag; consider rate limiting at the infrastructure level

### Recommended Deployment Practices

1. **Dedicated User**: Create a dedicated user account with minimal permissions
   ```bash
   sudo useradd -r -s /bin/false goa-agent
   sudo -u goa-agent goa spy ...
   ```

2. **Use Deploy Keys**: For GitHub/GitLab repositories, use read-only deploy keys instead of personal access tokens
   - Deploy keys are scoped to a single repository
   - They can be configured as read-only
   - Compromise exposes only one repository, not your entire account

3. **Sandboxing**: Consider running goa in a container or VM to limit blast radius
   ```bash
   docker run --read-only --user 1000:1000 goa spy ...
   ```

4. **Network Isolation**: Restrict network access from the goa host if possible

5. **Audit Logging**: Enable verbose logging (`-v 2`) in production to track executed commands

### Supply Chain Considerations

- Verify binary checksums from releases
- Review `.goa` files in repositories before enabling monitoring
- Consider using signed commits to verify repository integrity
