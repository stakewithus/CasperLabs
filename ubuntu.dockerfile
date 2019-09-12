FROM ubuntu:18.04

RUN apt-get update -y && apt-get install curl gnupg2 systemd git -y

RUN echo "deb https://dl.bintray.com/casperlabs/debian /" | tee -a /etc/apt/sources.list.d/casperlabs.list
RUN curl -o casperlabs-public.key.asc https://bintray.com/user/downloadSubjectPublicKey?username=casperlabs
RUN apt-key add casperlabs-public.key.asc
RUN apt-get update -y
RUN apt-get install casperlabs-node -y
RUN apt-get install casperlabs -y

RUN mkdir -p /opt/casper-deps
WORKDIR /opt/casper-deps

RUN apt-get install make build-essential -y

RUN apt-get install bsdmainutils -y

RUN git clone https://github.com/maandree/libkeccak.git; \
    git clone https://github.com/maandree/sha3sum.git; \
    cd libkeccak; git checkout 1.2; make install; ldconfig; \
    cd ../sha3sum; make;

RUN mkdir -p /usr/local/include
RUN cp /opt/casper-deps/libkeccak/libkeccak.so /usr/local/lib/libkeccak.so.1.2

RUN ln -sf -- "/usr/local/lib/libkeccak.so.1.2" "/usr/local/lib/libkeccak.so.1" && ln -sf -- "/usr/local/lib/libkeccak.so.1.2" "/usr/local/lib/libkeccak.so"

RUN cp /opt/casper-deps/libkeccak/libkeccak.a /usr/local/lib/libkeccak.a
RUN cp /opt/casper-deps/libkeccak/libkeccak.h /usr/local/include
RUN cp /opt/casper-deps/libkeccak/libkeccak-legacy.h /usr/local/include
RUN cp /opt/casper-deps/sha3sum/keccak-256sum /usr/local/bin

ADD ./hack /opt/casper-hack

WORKDIR /opt/casper-hack
