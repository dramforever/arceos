# How to run

## Build `payload/initrd.cpio`

Apply [the patch](0001-Add-__axmusl_handler.patch) to musl `v1.2.3`, and build it

```console
$ mkdir payload ; cd payload
$ cp <path-to-musl>/lib/libc.so ld.so
$ cp <path-to-hello>/bin/hello hello
$ ls ld.so hello | cpio --format=newc -o > initrd.cpio
$ cd ..
```

## Running the loader app:

```console
$ make ARCH=riscv64 A=apps/loader run
```

# References

- David Drysdale, *How programs get run: ELF binaries* https://lwn.net/Articles/631631/
