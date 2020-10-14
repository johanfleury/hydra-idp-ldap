#
# Builder
#

FROM rust as builder

WORKDIR /src/

RUN rustup default nightly

RUN apt-get -qq update && apt-get -qq install pysassc

COPY . .

RUN cargo +nightly build --release \
    && mkdir -p /install/ /install/assets/ \
    && cp target/release/hydra-idp-ldap /install/ \
    && cp -r assets/static/ /install/assets/ \
    && cp -r assets/templates/ /install/assets/ \
    && pysassc --sourcemap -t compressed assets/scss/main.scss /install/assets/static/css/main.css

#
# Actual image
#

FROM gcr.io/distroless/cc

COPY --from=builder /install/ /

ENTRYPOINT ["/hydra-idp-ldap"]
