FROM clux/muslrust:1.58.1-stable as builder
WORKDIR /volume
COPY . .
RUN cargo build --release

FROM alpine
# Copy the compiled binary from the builder container
COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/goa .
# Pass all arguments etc to binary
ENTRYPOINT [ "/goa" ]
