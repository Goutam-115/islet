#!/bin/sh

rm -f checkpoint/model.ckpt

HOST=$1
PORT=$2
MODEL_TYPE=$3   # "word" or "code"
IS_FL=$4        # 0 for ML, 1 for FL

GUI_RX_PORT=-1
if [ "$5" ]; then
  GUI_RX_PORT=$5
fi

GUI_TX_PORT=-1
if [ "$6" ]; then
  GUI_TX_PORT=$6
fi

./device.exe --print_all=true --operation=run-shell --data_dir=./data/ \
      --policy_store_file=policy_store --runtime_host="${HOST}" --runtime_data_port=${PORT} \
      --model_type="${MODEL_TYPE}" --is_fl=${IS_FL} \
      --gui_rx_port=${GUI_RX_PORT} --gui_tx_port=${GUI_TX_PORT}

