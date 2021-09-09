FROM rust:1.54.0-buster as builder
WORKDIR /usr/src
COPY . .
RUN cargo build --jobs 1 --release
RUN strip ./target/release/npm-package-explorer

FROM gcr.io/distroless/cc-debian10
WORKDIR /usr/dist
COPY --from=builder /usr/src/target/release/npm-package-explorer npm-package-explorer
COPY static static
ENTRYPOINT ["/usr/dist/npm-package-explorer"]
EXPOSE 8080
