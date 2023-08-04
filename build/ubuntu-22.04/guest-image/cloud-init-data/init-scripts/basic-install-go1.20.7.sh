#!/bin/bash

if [[ -d /usr/local/go ]]; then
    rm -rf /usr/local/go
fi
tar -C /usr/local -xzf /root/go1.20.7.linux-amd64.tar.gz
echo "export PATH=$PATH:/usr/local/go/bin" >> /etc/profile
rm /root/go1.20.7.linux-amd64.tar.gz