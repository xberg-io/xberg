```c title="C"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* The xberg C FFI does not bundle an MCP client. Drive the xberg
 * CLI's stdio MCP transport from a C host that also links libxberg. */
int main(void) {
    int request_pipe[2];
    int response_pipe[2];
    if (pipe(request_pipe) < 0 || pipe(response_pipe) < 0) {
        perror("pipe");
        return 1;
    }

    pid_t pid = fork();
    if (pid < 0) {
        perror("fork");
        return 1;
    }
    if (pid == 0) {
        dup2(request_pipe[0], 0);
        dup2(response_pipe[1], 1);
        close(request_pipe[1]);
        close(response_pipe[0]);
        execlp("xberg", "xberg", "mcp", (char *)NULL);
        perror("execlp");
        _exit(127);
    }

    close(request_pipe[0]);
    close(response_pipe[1]);

    const char *request =
        "{\"method\":\"tools/call\","
        "\"params\":{\"name\":\"extract\","
        "\"arguments\":{\"path\":\"document.pdf\",\"async\":true}}}\n";
    if (write(request_pipe[1], request, strlen(request)) < 0) {
        perror("write");
        return 1;
    }
    close(request_pipe[1]);

    char buffer[4096];
    ssize_t bytes_read = read(response_pipe[0], buffer, sizeof(buffer) - 1);
    if (bytes_read > 0) {
        buffer[bytes_read] = '\0';
        printf("%s", buffer);
    }
    close(response_pipe[0]);
    return 0;
}
```

<!-- snippet:syntax-only --> No MCP client is exposed by libxberg; this snippet drives the MCP CLI over stdio.
