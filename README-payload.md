Apply `0001-Add-__axmusl_handler.patch` to musl `v1.2.3`, and build `initrd.cpio`

```console
$ cp <path-to-musl>/lib/libc.so ld.so
$ cp <path-to-hello>/bin/hello hello
$ ls ld.so hello | cpio --format=newc -o > initrd.cpio
```
