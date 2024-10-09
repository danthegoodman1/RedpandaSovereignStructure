# use like: ./new-transform.sh <name>
mkdir -p transforms/$1
docker run --rm -w /transforms -v "$(pwd)/transforms/$1:/transforms" docker.redpanda.com/redpandadata/redpanda:v24.2.6 \
   transform init --language=rust --name=$1 --install-deps=false
