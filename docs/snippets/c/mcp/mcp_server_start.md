```c title="C"
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

/* The kreuzberg C FFI does not embed the MCP server. Spawn the kreuzberg
 * CLI from a host process that uses libkreuzberg for in-process extraction. */
int main(void) {
    pid_t pid = fork();
    if (pid < 0) {
        perror("fork");
        return 1;
    }
    if (pid == 0) {
        execlp("kreuzberg", "kreuzberg", "mcp", (char *)NULL);
        perror("execlp");
        _exit(127);
    }

    int status = 0;
    if (waitpid(pid, &status, 0) < 0) {
        perror("waitpid");
        return 1;
    }
    return WIFEXITED(status) ? WEXITSTATUS(status) : 1;
}
```

<!-- snippet:syntax-only --> The MCP server is exposed only through the kreuzberg CLI; libkreuzberg's C FFI offers no MCP entry point. This snippet spawns the CLI from a host that already links against libkreuzberg.
