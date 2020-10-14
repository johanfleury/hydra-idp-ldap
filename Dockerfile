FROM gcr.io/distroless/cc

COPY build/ /

ENTRYPOINT ["/hydra-idp-ldap"]
