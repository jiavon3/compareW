#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

/* Redirect host GTK config so UOS /etc/gtk-3.0/settings.ini cannot load
 * GLib 2.58 gtk-modules (gail/atk-bridge) into bundled GTK. */

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
