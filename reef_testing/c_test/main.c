void reef_log(char *ptr, int bytes_len)
    __attribute__((__import_module__("reef"), __import_name__("log"), ));

int reef_strlen(char *ptr) {
  int len = 0;
  while (ptr && ptr[len] != '\0') {
    len++;
  }

  return len;
}

int reef_main() {
  char *msg = "Hello World!";
  int len = reef_strlen(msg);

  reef_log(msg, len);

  return 42;
}
