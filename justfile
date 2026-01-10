capnp-version := "1.2.0"

# list all available just recipes
list:
    @ just --list --unsorted

# build and install capnp from source
[unix]
install-capnp:
    curl -O https://capnproto.org/capnproto-c++-{{capnp-version}}.tar.gz
    tar zxf capnproto-c++-{{capnp-version}}.tar.gz
    rm capnproto-c++-{{capnp-version}}.tar.gz

    cd capnproto-c++-{{capnp-version}} && \
      ./configure && \
      make -j6 check && \
      sudo make install
