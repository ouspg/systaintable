#!/bin/bash
set -e

echo "Building Lambda function with Docker..."

# Create a Dockerfile with a specialized cross-compilation image
cat > Dockerfile.lambda << 'EOF'
FROM messense/rust-musl-cross:x86_64-musl as builder

WORKDIR /home/rust/src

# Copy your project files
COPY Cargo.toml Cargo.lock ./
COPY src ./src/
# Don't copy .cargo config as it has macOS-specific paths

# Build the Lambda function
RUN cargo build --release --bin lambda

# The binary will be statically linked and ready for Lambda
RUN cp target/x86_64-unknown-linux-musl/release/lambda /home/rust/bootstrap

# Create minimal Lambda container
FROM public.ecr.aws/lambda/provided:al2
COPY --from=builder /home/rust/bootstrap /var/runtime/bootstrap
CMD [ "bootstrap" ]
EOF

# Build Docker image
docker build -t regex-classifier-lambda -f Dockerfile.lambda .

# Remove existing container if it exists
docker rm -f lambda-extract 2>/dev/null || true

# Extract the Lambda binary
docker create --name lambda-extract regex-classifier-lambda
mkdir -p lambda-package
docker cp lambda-extract:/var/runtime/bootstrap lambda-package/
docker rm lambda-extract

# Create deployment package
cd lambda-package
zip lambda.zip bootstrap
cd ..

echo "Lambda package created at: $(pwd)/lambda-package/lambda.zip"
echo "Ready to deploy to AWS Lambda!"