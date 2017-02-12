## -*- docker-image-name: "clma/eden" -*-
FROM stephank/archlinux:armv6-latest
MAINTAINER clma <claus@crate.io>

ENV EDEN_VER arm-latest
RUN pacman -Sy --noconfirm tar gzip  && pacman -Sc --noconfirm && rm -Rf /usr/share
RUN curl -s https://x5ff.xyz:8080/builds/eden-$EDEN_VER.tgz | tar xfz - && chmod +x /eden/eden
VOLUME ["/sensors"]

ENV PATH /eden:$PATH

CMD ["eden", "-c /eden/config.toml", "-l /eden/logging.yml"]
