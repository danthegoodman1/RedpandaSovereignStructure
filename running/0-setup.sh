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

cargo build --target=wasm32-wasip1 --release --package format --manifest-path transforms/format/Cargo.toml
mv transforms/format/target/wasm32-wasip1/release/format.wasm transforms/format/
docker compose exec -w /transforms/format redpanda-0 rpk transform deploy

echo "Transforms compiled"

echo "Setup complete"
