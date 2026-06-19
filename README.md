## Dumb Voice Protocol

# Intro

This is a CLI program that reads your microphone device and outputs information to stdout / ipc.

# Usage

```
dmvop --output=stdout --fmt="%{vol},%{word},%{confidence}" --device="/dev/mymic"
      --output=stderr --fmt-file="./fmt.txt"
      --output=ipc
      --output=tcp
      --output=udp
```

# License

Under WTFPL
