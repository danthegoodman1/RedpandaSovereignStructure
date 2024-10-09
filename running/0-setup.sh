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

build_and_deploy_transform() {
    cargo build --target=wasm32-unknown-wasi --release --package $1 --manifest-path $2/Cargo.toml
    mv $2/target/wasm32-unknown-wasi/release/$1.wasm $2/
    docker compose exec -w /transforms/$2 redpanda-0 rpk transform deploy
}

build_and_deploy_transform format transforms/format

echo "Transforms compiled"

echo "Setup complete"
