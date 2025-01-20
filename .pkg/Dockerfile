FROM archlinux:base-devel AS build

COPY PKGBUILD webnote.service webnote rootfs/

RUN useradd b
RUN chown -R b rootfs

RUN sed '/^OPTIONS=/s/debug/!debug/' -i /etc/makepkg.conf

USER b
WORKDIR /rootfs
RUN makepkg -s

FROM scratch
COPY --from=build /rootfs/*.pkg.tar.zst /
