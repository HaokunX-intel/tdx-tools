#!/bin/bash

cd /root/
pip3 install intel-tensorflow-avx512==2.11.0
wget https://storage.googleapis.com/intel-optimized-tensorflow/models/v2_5_0/dien_bf16_pretrained_opt_model.pb
wget https://storage.googleapis.com/intel-optimized-tensorflow/models/v2_5_0/dien_fp32_static_rnn_graph.pb

mkdir dien
pushd dien
wget https://zenodo.org/record/3463683/files/data.tar.gz
wget https://zenodo.org/record/3463683/files/data1.tar.gz
wget https://zenodo.org/record/3463683/files/data2.tar.gz

tar -jxvf data.tar.gz
mv data/* .
tar -jxvf data1.tar.gz
mv data1/* .
tar -jxvf data2.tar.gz
mv data2/* .
popd

git clone https://github.com/IntelAI/models.git
pushd models
git checkout v2.5.0
popd