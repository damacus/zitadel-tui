## 2025-02-12 - Command Injection in Ruby System Calls
**Vulnerability:** Shell command injection via interpolated strings in system calls (`\``` and `TTY::Command.new.run("string")`).
**Learning:** Using backticks or passing a single string to a command execution method invokes the system shell, which evaluates shell metacharacters. Even if user input is not directly passed initially, configuration or secret keys containing characters like `;`, `&`, or `|` can lead to arbitrary code execution.
**Prevention:** Always use an array of arguments for system calls (e.g., `TTY::Command.new.run('cmd', 'arg1', 'arg2')` or `system('cmd', 'arg1')`) to bypass the shell and pass arguments directly to the executable.
