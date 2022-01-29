FROM clux/muslrust@sha256:211e3420f0d5faae267e0906cf81ce680078aab818ecef7ef9749ff0f521218f as builder
WORKDIR /volume
COPY . .
RUN cargo build --release

FROM alpine@sha256:2689e157117d2da668ad4699549e55eba1ceb79cb7862368b30919f0488213f4
# Copy the compiled binary from the builder container
COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/goa .
# Pass all arguments etc to binary
ENTRYPOINT [ "/goa" ]
