echo "Setting up the environment..."

docker compose down -v
docker compose up -d

# Create the topics
echo "Creating topics..."
docker compose exec redpanda-0 rpk topic create input

docker compose exec redpanda-0 rpk topic create unprocessed

docker compose exec redpanda-0 rpk topic create unverified

docker compose exec redpanda-0 rpk topic create unprocessable

docker compose exec redpanda-0 rpk topic create verified

echo "Topics created"

# Add schemas
echo "Adding schemas..."
docker compose exec redpanda-0 rpk registry schema create record_attempted --schema /schemas/record_attempted.json
echo "Schemas added"

# Enable transforms
echo "Enabling transforms..."
docker compose exec redpanda-0 rpk cluster config set data_transforms_enabled true
echo "Transforms enabled"
echo "Restarting the cluster..."
docker compose down
docker compose up -d
echo "Cluster restarted"


# Compile transforms
echo "Compiling transforms..."
rustup target add wasm32-wasip1 # need this!

build_and_deploy_transform() {
    RUSTFLAGS=-Ctarget-feature=+simd128 cargo build --target=wasm32-wasip1 --release --package $1 --manifest-path transforms/$1/Cargo.toml
    mv transforms/$1/target/wasm32-wasip1/release/$1.wasm transforms/$1/
    docker compose exec -w /transforms/$1 redpanda-0 rpk transform deploy
}

build_and_deploy_transform format transforms/format
build_and_deploy_transform validation transforms/validation

echo "Transforms compiled"

echo "Setup complete"
