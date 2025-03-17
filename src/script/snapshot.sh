#/bin/bash

echo $#
if [ $# -lt 2 ]; then
		echo "Usage: $0 <outdir> (mainnet|testnet3|testnet4|signet|regtest) [<rest_host=http://localhost> [<bitcoin_dir=$HOME/.bitcoin>]]"
		exit 1
fi

OUTDIR=$1
NETWORK=$2
if [ $# -ge 3 ]; then
	REST_HOST=$3
else
	REST_HOST=http://localhost
fi
if [ $# -ge 4 ]; then
	BITCOIN_DIR=$4
else
	BITCOIN_DIR=$HOME/.bitcoin
fi

if [ "$NETWORK" == "mainnet" ]; then
	RPC_PORT=8332
	BITCOIN_BLOCKS_DIR="${BITCOIN_DIR}/blocks"
elif [ "$NETWORK" == "testnet3" ]; then
	RPC_PORT=18332
	BITCOIN_BLOCKS_DIR="${BITCOIN_DIR}/testnet3/blocks"
elif [ "$NETWORK" == "testnet4" ]; then
	RPC_PORT=48332
	BITCOIN_BLOCKS_DIR="${BITCOIN_DIR}/testnet4/blocks"
elif [ "$NETWORK" == "signet" ]; then
	RPC_PORT=38332
	BITCOIN_BLOCKS_DIR="${BITCOIN_DIR}/signet/blocks"
elif [ "$NETWORK" == "regtest" ]; then
	RPC_PORT=18443
	BITCOIN_BLOCKS_DIR="${BITCOIN_DIR}/regtest/blocks"
else
	echo "Invalid network: $NETWORK"
	exit 1
fi

REST_URL="${REST_HOST}:${RPC_PORT}/rest"

# Fetch the best height.
BEST_HEIGHT=$(curl -s "${REST_URL}/chaininfo.json" | jq .blocks)
echo "Best height: $BEST_HEIGHT"

BATCH_BLOCK_COUNT=100000
for START_HEIGHT in $(seq 0 $BATCH_BLOCK_COUNT $BEST_HEIGHT); do
	END_HEIGHT=$((START_HEIGHT + BATCH_BLOCK_COUNT - 1))
	# Determine if it is the last batch.
	if [ ${END_HEIGHT} -gt $BEST_HEIGHT ]; then
		IS_LAST=1
	else
		IS_LAST=0
	fi
	OUTFILE="${OUTDIR}/bootstrap-${NETWORK}_$(printf "%02d" $(($START_HEIGHT / $BATCH_BLOCK_COUNT))).dat.zst"
	if [ $IS_LAST -eq 0 ]; then
		if [ -f $OUTFILE ]; then
			echo "Skipping $OUTFILE"
			continue
		fi
	fi
	echo "Processing ${START_HEIGHT}...${END_HEIGHT}"
	cargo run --release --bin gen_bootstrap_dat "$REST_URL" "$BITCOIN_BLOCKS_DIR" /dev/stdout $START_HEIGHT $END_HEIGHT | zstd -T4 -o $OUTFILE
done
