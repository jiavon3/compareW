#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

/* Redirect host GTK config and refuse host GTK/GIO plugins. UOS DDE injects
 * atk-bridge via XSETTINGS even when /etc/gtk-3.0/settings.ini is redirected. */

static const char *redirect_abs(const char *path) {
  if (path == NULL) {
    return NULL;
  }
  if (strcmp(path, "/etc/gtk-3.0/settings.ini") == 0 ||
      strcmp(path, "/etc/xdg/gtk-3.0/settings.ini") == 0) {
    return "/opt/comparew/usr/etc/gtk-3.0/settings.ini";
  }
  if (strcmp(path, "/usr/lib/x86_64-linux-gnu/gtk-3.0/3.0.0/immodules.cache") == 0 ||
      strcmp(path, "/usr/lib/gtk-3.0/3.0.0/immodules.cache") == 0) {
    return "/opt/comparew/usr/lib/gtk-3.0/3.0.0/immodules.cache";
  }
  if (strcmp(path,
             "/usr/lib/x86_64-linux-gnu/gdk-pixbuf-2.0/2.10.0/loaders.cache") == 0 ||
      strcmp(path, "/usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache") == 0) {
    return "/opt/comparew/usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache";
  }
  if (strcmp(path, "/usr/lib/x86_64-linux-gnu/gio/modules") == 0 ||
      strcmp(path, "/usr/lib/gio/modules") == 0) {
    return "/opt/comparew/usr/lib/gio/modules";
  }
  return path;
}

static const char *redirect_at(int dirfd, const char *path) {
  char fdpath[64];
  char dirbuf[512];
  ssize_t n;

  if (path != NULL && path[0] == '/') {
    return redirect_abs(path);
  }
  if (path == NULL || strcmp(path, "settings.ini") != 0 || dirfd == AT_FDCWD) {
    return path;
  }
  snprintf(fdpath, sizeof fdpath, "/proc/self/fd/%d", dirfd);
  n = readlink(fdpath, dirbuf, sizeof dirbuf - 1);
  if (n <= 0) {
    return path;
  }
  dirbuf[n] = '\0';
  if (strcmp(dirbuf, "/etc/gtk-3.0") == 0 || strcmp(dirbuf, "/etc/xdg/gtk-3.0") == 0) {
    return "/opt/comparew/usr/etc/gtk-3.0/settings.ini";
  }
  return path;
}

static int has_substr(const char *s, const char *needle) {
  return s != NULL && needle != NULL && strstr(s, needle) != NULL;
}

static int should_block_dlopen(const char *filename) {
  const char *base;

  if (filename == NULL || filename[0] == '\0') {
    return 0;
  }
  if (strncmp(filename, "/opt/comparew/", 14) == 0) {
    return 0;
  }
  base = strrchr(filename, '/');
  base = base != NULL ? base + 1 : filename;

  if (has_substr(base, "atk-bridge") || has_substr(base, "libgail") ||
      strcmp(base, "gail") == 0 || has_substr(base, "canberra") ||
      has_substr(base, "overlay-scrollbar") || has_substr(base, "unity-gtk") ||
      has_substr(base, "appmenu") || has_substr(base, "gtk3-nocsd") ||
      has_substr(base, "im-ibus") || has_substr(base, "im-fcitx") ||
      has_substr(base, "libibus") || has_substr(base, "gvfs") ||
      has_substr(base, "dconfsettings") || has_substr(base, "gioremote") ||
      has_substr(base, "giognomeproxy") || has_substr(base, "giolibproxy")) {
    return 1;
  }
  if (filename[0] == '/' && strncmp(filename, "/opt/", 5) != 0) {
    if (has_substr(filename, "/gtk-3.0/") || has_substr(filename, "/gio/modules") ||
        has_substr(filename, "/gdk-pixbuf")) {
      return 1;
    }
  }
  return 0;
}

int open(const char *path, int flags, ...) {
  static int (*real_open)(const char *, int, ...) = NULL;
  mode_t mode = 0;
  if (real_open == NULL) {
    real_open = (int (*)(const char *, int, ...))dlsym(RTLD_NEXT, "open");
  }
  path = redirect_abs(path);
  if (flags & O_CREAT) {
    va_list ap;
    va_start(ap, flags);
    mode = (mode_t)va_arg(ap, int);
    va_end(ap);
    return real_open(path, flags, mode);
  }
  return real_open(path, flags);
}

int open64(const char *path, int flags, ...) {
  static int (*real_open64)(const char *, int, ...) = NULL;
  mode_t mode = 0;
  if (real_open64 == NULL) {
    real_open64 = (int (*)(const char *, int, ...))dlsym(RTLD_NEXT, "open64");
  }
  path = redirect_abs(path);
  if (flags & O_CREAT) {
    va_list ap;
    va_start(ap, flags);
    mode = (mode_t)va_arg(ap, int);
    va_end(ap);
    return real_open64(path, flags, mode);
  }
  return real_open64(path, flags);
}

int openat(int dirfd, const char *path, int flags, ...) {
  static int (*real_openat)(int, const char *, int, ...) = NULL;
  mode_t mode = 0;
  if (real_openat == NULL) {
    real_openat = (int (*)(int, const char *, int, ...))dlsym(RTLD_NEXT, "openat");
  }
  path = redirect_at(dirfd, path);
  if (flags & O_CREAT) {
    va_list ap;
    va_start(ap, flags);
    mode = (mode_t)va_arg(ap, int);
    va_end(ap);
    return real_openat(dirfd, path, flags, mode);
  }
  return real_openat(dirfd, path, flags);
}

FILE *fopen(const char *path, const char *mode) {
  static FILE *(*real_fopen)(const char *, const char *) = NULL;
  if (real_fopen == NULL) {
    real_fopen = (FILE *(*)(const char *, const char *))dlsym(RTLD_NEXT, "fopen");
  }
  return real_fopen(redirect_abs(path), mode);
}

void *dlopen(const char *filename, int flags) {
  static void *(*real_dlopen)(const char *, int) = NULL;
  if (real_dlopen == NULL) {
    real_dlopen = (void *(*)(const char *, int))dlsym(RTLD_NEXT, "dlopen");
  }
  if (should_block_dlopen(filename)) {
    fprintf(stderr, "comparew-redirect: blocked dlopen %s\n", filename);
    return NULL;
  }
  return real_dlopen(filename, flags);
}
