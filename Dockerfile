FROM clux/muslrust@sha256:4d4ea969412a784a545444f4cf8f7ae2436cc4af04151a6e98a081c49326fc17 AS builder
WORKDIR /volume
COPY . .
RUN cargo build --release

FROM alpine@sha256:25109184c71bdad752c8312a8623239686a9a2071e8825f20acb8f2198c3f659
# Copy the compiled binary from the builder container
COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/goa .
# Pass all arguments etc to binary
ENTRYPOINT [ "/goa" ]
