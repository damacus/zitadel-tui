## 2025-02-12 - Command Injection in Ruby System Calls
**Vulnerability:** Shell command injection via interpolated strings in system calls (`\``` and `TTY::Command.new.run("string")`).
**Learning:** Using backticks or passing a single string to a command execution method invokes the system shell, which evaluates shell metacharacters. Even if user input is not directly passed initially, configuration or secret keys containing characters like `;`, `&`, or `|` can lead to arbitrary code execution.
**Prevention:** Always use an array of arguments for system calls (e.g., `TTY::Command.new.run('cmd', 'arg1', 'arg2')` or `system('cmd', 'arg1')`) to bypass the shell and pass arguments directly to the executable.
## 2024-03-05 - Insecure File Write and Symlink Vulnerability in Credentials Fetching
**Vulnerability:** The application was downloading a service account key and writing it to `/tmp/zitadel-sa.json` using `File.write` with default permissions, making the sensitive credentials readable by other users. Furthermore, it used `File.exist?` before writing the file, making it vulnerable to symlink attacks where an attacker could create a dangling symlink at the destination path, causing the application to overwrite an arbitrary file on the system.
**Learning:** Checking `File.exist?` does not protect against dangling symlinks. Using `File.write` without explicit `perm` arguments on sensitive files creates them with default umask permissions, which is often insecure.
**Prevention:** Always check for symlinks using `File.symlink?` before checking existence or overwriting files in predictable temporary locations. Always specify restricted permissions (e.g., `perm: 0o600`) when writing sensitive data to disk.
## 2024-05-24 - Avoid Storing Sensitive Files in /tmp

**Vulnerability:** The default service account key file path was set to `/tmp/zitadel-sa.json`.
**Learning:** Storing sensitive credentials in a world-writable directory like `/tmp` exposes them to unauthorized access, privilege escalation, or symlink attacks by other users on the same system.
**Prevention:** Store sensitive configuration and credential files in user-specific, restricted directories (e.g., `~/.zitadel-sa.json`) instead of shared temporary directories.
