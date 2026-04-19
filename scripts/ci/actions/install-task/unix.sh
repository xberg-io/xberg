#!/usr/bin/env bash
set -euo pipefail

version="${1:?version required}"

task_bin_dir="${HOME}/.local/bin"
mkdir -p "$task_bin_dir"

install_task() {
  local max_attempts=5
  local attempt=1
  local wait_time=2

  while [[ $attempt -le $max_attempts ]]; do
    echo "Installing Task v${version} (attempt ${attempt}/${max_attempts})..."

    # Download the install script with timeout and retries
    if curl --location \
      --connect-timeout 10 \
      --max-time 60 \
      --retry 5 \
      --retry-delay 2 \
      --retry-all-errors \
      https://taskfile.dev/install.sh | sh -s -- -d -b "$task_bin_dir"; then

      # Verify that the task binary exists and is executable
      if [[ -x "$task_bin_dir/task" ]]; then
        echo "Task installation successful"
        return 0
      else
        echo "Error: Task binary not found at $task_bin_dir/task"
        rm -f "$task_bin_dir/task"
        attempt=$((attempt + 1))
        if [[ $attempt -le $max_attempts ]]; then
          echo "Retrying in ${wait_time}s..."
          sleep "$wait_time"
          wait_time=$((wait_time * 2))
        fi
      fi
    else
      echo "Download/installation failed"
      attempt=$((attempt + 1))
      if [[ $attempt -le $max_attempts ]]; then
        echo "Retrying in ${wait_time}s..."
        sleep "$wait_time"
        wait_time=$((wait_time * 2))
      fi
    fi
  done

  echo "Error: Failed to install Task after ${max_attempts} attempts" >&2
  return 1
}

if ! command -v task >/dev/null 2>&1 || [[ "$(task --version 2>/dev/null || echo '')" != *"$version"* ]]; then
  install_task
fi

# Final verification before adding to PATH
if [[ ! -x "$task_bin_dir/task" ]]; then
  echo "Error: Task binary not found or not executable at $task_bin_dir/task" >&2
  exit 1
fi

echo "Task is ready at $task_bin_dir/task"
echo "$task_bin_dir" >>"$GITHUB_PATH"
