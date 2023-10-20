# Ek-tool

The ek-tool is used to retrieve ek certification from the vtpm.

## Building

### Prerequisite

The ek-tool is built on some dynamic libraries from dcap 1.18. 

We show steps to install dependencies on the Ubuntu 22.04.

1. Install sgx sdk
    
    ```
    wget https://download.01.org/intel-sgx/sgx-dcap/1.18/linux/distro/ubuntu22.04-server/sgx_linux_x64_sdk_2.21.100.1.bin

    chmod 755 sgx_linux_x64_sdk_2.21.100.1.bin

    sudo ./sgx_linux_x64_sdk_2.21.100.1.bin
    ```
    
    Install the sdk to `/opt/intel`

2. Install dcap libraries

    Setup dcap repo.

    ```
    wget https://download.01.org/intel-sgx/sgx-dcap/1.18/linux/distro/ubuntu22.04-server/sgx_debian_local_repo.tgz

    tar xvf sgx_debian_local_repo.tgz

    cp -r sgx_debian_local_repo /srv

    cat <<EOF >> /etc/apt/sources.list.d/sgx_debian_local_repo.list
    deb [trusted=yes arch=amd64] file:/srv/sgx_debian_local_repo jammy main
    EOF
    ```

    Install packages.

    ```
    sudo apt install -y tdx-qgs \
        libsgx-dcap-default-qpl \
        libsgx-dcap-default-qpl-dev \
        libsgx-dcap-quote-verify \
        libsgx-dcap-quote-verify-dev \
        libsgx-ae-qve
    ```

3. Install `clang`
    The rust cargo of dcap built by `clang`.

    ```
    sudo apt install -y clang
    ```

### Build ek-tool

Use the makefile to build the binary.

```
source /opt/intel/sgxsdk/environment

cd ek-tool

make

ls target/release/ek-tool
```


## Runtime

### Host Env

The ek-tool needs to access the service `sgx-dcap-pccs` to verify quote before retrieving ek certification. Therefore make sure the  `sgx-dcap-pccs` can be accessed on some servers.

If the service sgx-dcap-pccs resides on the host, the `hosts` entry in the config should be set to `"0.0.0.0"`.

```
cat /opt/intel/sgx-dcap-pccs/config/default.json

{
    ...
    "hosts" : "0.0.0.0",
    ...
}

```

### Guest Env

Install dcap libraries in the by following steps in above sections.

Install tpm2-tools to help ek-tool to retrieve data from vtpm.

```
apt install -y tpm2-tools
```

Set the host ip `<host_ip>` and port `<host_port>` of the service sgx-dcap-pccs in config file `/etc/sgx_default_qcnl.conf`.

```
cat /etc/sgx_default_qcnl.conf 

{
  "pccs_url": "https://<host_ip>:<host_port>/sgx/certification/v4/"
  ,"use_secure_cert": false
  ...
}
```

*Note: if the guest has some proxies and `<host_ip>` can not be accessed throght the proxies, append the `<host_ip>` in the `no_proxy` or `NO_PROXY`.*

### Example 

1. EK Certification Retrieve

The ek-tool will retrieve the ek public key and out it in base64 in the stdout.

```
sudo ./ek-tool

# ek_pub_base64: <ek_pub_base64>
```

2. Verify the provided CA from vtpm

```
sudo ./ek-tool -c <ek_cert_base64>

# verify_provided_ca: true
```

Use option `c` to provide a base64 string of the ca cert.
