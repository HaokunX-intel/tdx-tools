#!/bin/bash
#
# Create a Ubuntu EFI cloud TDX guest image. It can run on any Linux system with
# required tool installed like qemu-img, virt-customize, virt-install, etc. It is
# not required to run on a TDX capable system.
#

CURR_DIR=$(dirname "$(realpath $0)")
FORCE_RECREATE=false
TEST_SUITE=false
OFFICIAL_UBUNTU_IMAGE="https://cloud-images.ubuntu.com/jammy/current/"
CLOUD_IMG="jammy-server-cloudimg-amd64.img"
GUEST_IMG="tdx-guest-ubuntu-22.04.qcow2"
SIZE=20
GUEST_USER="tdx"
GUEST_PASSWORD="123456"
GUEST_HOSTNAME="tdx-guest"
KERNEL_VERSION=""
GUEST_REPO=""
HOST_REPO=""
DEBUG_MODE=false
AUTH_FILE=""

ok() {
    echo -e "\e[1;32mSUCCESS: $*\e[0;0m"
}

error() {
    echo -e "\e[1;31mERROR: $*\e[0;0m"
    cleanup
    exit 1
}

warn() {
    echo -e "\e[1;33mWARN: $*\e[0;0m"
}

check_tool() {
    [[ "$(command -v $1)" ]] || { error "$1 is not installed" 1>&2 ; }
}

usage() {
    cat <<EOM
Usage: $(basename "$0") [OPTION]...
Required
  -r <guest repo>           Specify the directory including guest packages, generated by build-repo.sh or remote repo
Test suite
  -t                        Install test suite package
  -e <host repo>            Specify the directory including host packages, generated by build-repo.sh or remote repo
Optional
  -d                        Debug mode:
                                * enable ssh root user login
  -v <kernel version>       Specify the version of the guest kernel, like 6.2.16-mvp30v3+7-generic of
                            linux-image-unsigned-6.2.16-mvp30v3+7-generic. If the guest repo is remote,
                            the option is necessary. 
  -a                        Auth file that will be placed in /etc/apt/auth.conf.d
  -h                        Show this help
  -f                        Force to recreate the output image
  -n                        Guest host name, default is "tdx-guest"
  -u                        Guest user name, default is "tdx"
  -p                        Guest password, default is "123456"
  -s                        Specify the size of guest image
  -o <output file>          Specify the output file, default is tdx-guest-ubuntu-22.04.qcow2.
                            Please make sure the suffix is qcow2. Due to permission consideration,
                            the output file will be put into /tmp/<output file>.
EOM
}

process_args() {
    while getopts "o:s:n:u:p:r:e:a:v:fhtd" option; do
        case "$option" in
        o) GUEST_IMG=$OPTARG ;;
        s) SIZE=$OPTARG ;;
        n) GUEST_HOSTNAME=$OPTARG ;;
        u) GUEST_USER=$OPTARG ;;
        p) GUEST_PASSWORD=$OPTARG ;;
        r) GUEST_REPO=$OPTARG ;;
        e) HOST_REPO=$OPTARG ;;
        a) AUTH_FILE=$OPTARG ;;
        v) KERNEL_VERSION=$OPTARG ;;
        f) FORCE_RECREATE=true ;;
        t) TEST_SUITE=true;;
        d) DEBUG_MODE=true;;
        h)
            usage
            exit 0
            ;;
        *)
            echo "Invalid option '-$OPTARG'"
            usage
            exit 1
            ;;
        esac
    done

    echo "================================="
    echo "Guest image /tmp/${GUEST_IMG}"
    echo "Built from ${OFFICIAL_UBUNTU_IMAGE}${CLOUD_IMG}"
    echo "Guest package installed from ${GUEST_REPO}"
    echo "Host package installed from ${HOST_REPO}"
    echo "Force recreate:   ${FORCE_RECREATE}"
    echo "Debug mode:       ${DEBUG_MODE}"
    echo "Test suite:       ${TEST_SUITE}"
    echo "Kernel version:   ${KERNEL_VERSION}"
    echo "Size:             ${SIZE}G"
    echo "Hostname:         ${GUEST_HOSTNAME}"
    echo "User:             ${GUEST_USER}"
    echo "Password:         ******"
    echo "================================="

    if [[ "${GUEST_IMG}" == "${CLOUD_IMG}" ]]; then
        error "Please specify a different name for guest image via -o"
    fi

    if [[ ${GUEST_IMG} != *.qcow2 ]]; then
        error "The output file should be qcow2 format with the suffix .qcow2."
    fi

    if [[ -f "/tmp/${GUEST_IMG}" ]]; then
        if [[ ${FORCE_RECREATE} != "true" ]]; then
            error "Guest image /tmp/${GUEST_IMG} already exist, please specify -f if want force to recreate"
        fi
    fi

    if [[ ${GUEST_REPO} != 'http:'* ]] && [[ ${GUEST_REPO} != 'https:'* ]] && [[ ${GUEST_REPO} != 'ftp:'* ]];then 
        if [[ -z ${GUEST_REPO} ]]; then
            error "No guest repository provided, skip to install TDX packages..."
        else
            if [[ ! -d ${GUEST_REPO} ]]; then
                error "The guest repo directory ${GUEST_REPO} does not exists..."
            fi
        fi
    fi

    if [[ ${TEST_SUITE} == "true" ]]; then
        if [[ ! -d ${HOST_REPO} ]]; then
            error "The host repo directory ${GUEST_REPO} is necessary if install test suite"
        fi
    fi 

}

download_image() {
    # Get the checksum file first
    if [[ -f ${CURR_DIR}/"SHA256SUMS" ]]; then
        rm ${CURR_DIR}/"SHA256SUMS"
    fi

    wget "${OFFICIAL_UBUNTU_IMAGE}/SHA256SUMS"

    while :; do
        # Download the cloud image if not exists
        if [[ ! -f ${CLOUD_IMG} ]]; then
            wget -O ${CURR_DIR}/${CLOUD_IMG} ${OFFICIAL_UBUNTU_IMAGE}/${CLOUD_IMG}
        fi

        # calculate the checksum
        download_sum=$(sha256sum ${CURR_DIR}/${CLOUD_IMG} | awk '{print $1}')
        found=false
        while IFS= read -r line || [[ -n "$line" ]]; do
            if [[ "$line" == *"$CLOUD_IMG"* ]]; then
                if [[ "${line%% *}" != ${download_sum} ]]; then
                    echo "Invalid download file according to sha256sum, re-download"
                    rm ${CURR_DIR}/${CLOUD_IMG}
                else
                    ok "Verify the checksum for Ubuntu cloud image."
                    return
                fi
                found=true
            fi
        done <"SHA256SUMS"
        if [[ $found != "true" ]]; then
            echo "Invalid SHA256SUM file"
            exit 1
        fi
    done
}

create_guest_image() {
    download_image

    cp ${CURR_DIR}/${CLOUD_IMG} /tmp/${GUEST_IMG}
    ok "Copy the ${CLOUD_IMG} => /tmp/${GUEST_IMG}"
}

config_guest_env() {
    virt-customize -a /tmp/${GUEST_IMG} \
        --copy-in /etc/environment:/etc
    ok "Copy host's environment file to guest for http_proxy"
}

resize_guest_image() {
    qemu-img resize /tmp/${GUEST_IMG} +${SIZE}G
    virt-customize -a /tmp/${GUEST_IMG} \
        --run-command 'growpart /dev/sda 1' \
        --run-command 'resize2fs /dev/sda1' \
        --run-command 'systemctl mask pollinate.service'
    ok "Resize the guest image to ${SIZE}G"
}

config_cloud_init() {
    pushd ${CURR_DIR}/cloud-init-data
    [ -e /tmp/ciiso.iso ] && rm /tmp/ciiso.iso

    # configure the meta-dta
    cp meta-data.template meta-data

    cat <<EOT >> meta-data

local-hostname: $GUEST_HOSTNAME
EOT

    ok "Generate configuration for cloud-init..."
    genisoimage -output /tmp/ciiso.iso -volid cidata -joliet -rock user-data meta-data
    ok "Generate the cloud-init ISO image..."
    popd

    virt-install --memory 4096 --vcpus 4 --name tdx-config-cloud-init \
        --disk /tmp/${GUEST_IMG} \
        --disk /tmp/ciiso.iso,device=cdrom \
        --os-type Linux \
        --os-variant ubuntu21.10 \
        --virt-type kvm \
        --graphics none \
        --import 
    ok "Complete cloud-init..."
    sleep 1

    virsh destroy tdx-config-cloud-init || true
    virsh undefine tdx-config-cloud-init || true
}


install_tdx_measure_tool() {
    virt-customize -a /tmp/${GUEST_IMG} \
        --run-command "python3 -m pip install pytdxattest"
    ok "Install the TDX measurement tool..."
}


prepare_repos() {
    if [[ ${GUEST_REPO} != 'http:'* ]] && [[ ${GUEST_REPO} != 'https:'* ]] && [[ ${GUEST_REPO} != 'ftp:'* ]];then
        virt-customize -a /tmp/${GUEST_IMG} \
            --copy-in ${GUEST_REPO}:/srv/
    fi
    if [[ ! -z ${AUTH_FILE} ]]; then
        virt-customize -a /tmp/${GUEST_IMG} \
            --copy-in ${AUTH_FILE}:/etc/apt/auth.conf.d/
    fi
}

create_user_data() {
    GUEST_REPO_NAME=""
    guest_repo_source=""
    if [[ ${GUEST_REPO} == 'http:'* ]] || [[ ${GUEST_REPO} == 'https:'* ]] || [[ ${GUEST_REPO} == 'ftp:'* ]]; then 
        GUEST_REPO_NAME=$(basename realpath ${GUEST_REPO})
        guest_repo_source='deb [trusted=yes] '$GUEST_REPO'/ jammy/all/\ndeb [trusted=yes] '$GUEST_REPO'/ jammy/amd64/'
    else
        GUEST_REPO_NAME=$(basename $(realpath ${GUEST_REPO}))
        guest_repo_source='deb [trusted=yes] file:/srv/'$GUEST_REPO_NAME'/ jammy/all/\ndeb [trusted=yes] file:/srv/'$GUEST_REPO_NAME'/ jammy/amd64/'
        kernel=$(tree ${GUEST_REPO} | grep linux-image-unsigned | awk '{print $3}')
        KERNEL_VERSION=$(echo $kernel | awk -F'_' '{print $1}')
        KERNEL_VERSION=$(echo ${KERNEL_VERSION#linux-image-unsigned-})
    fi

    yq "
    .apt.sources.\"$GUEST_REPO_NAME.list\".source=\"$guest_repo_source\" |
    .packages[0]=\"linux-image-unsigned-$KERNEL_VERSION\" |
    .packages[1]=\"linux-modules-$KERNEL_VERSION\" |
    .packages[2]=\"linux-modules-extra-$KERNEL_VERSION\" |
    .packages[3]=\"linux-headers-$KERNEL_VERSION\" |
    .packages[4]=\"python3-pip\"
    " "${CURR_DIR}"/cloud-init-data/user-data-basic/cloud-config-base-template.yaml > \
    "${CURR_DIR}"/cloud-init-data/cloud-config-base.yaml


    # mergo multi-part input
    cloud-init devel make-mime \
        -a ./cloud-init-data/cloud-config-base.yaml:cloud-config\
        > ./cloud-init-data/user-data

    # HOST_REPO_NAME=$(basename $(realpath ${HOST_REPO}))

    # cloud-init devel make-mime \
    #     -a ./cloud-init-data/user-data-basic/cloud-config-base.yaml:cloud-config \
    #     -a ./cloud-init-data/user-data-customized/cloud-config-test-suite.yaml:cloud-config \
    #     > ./cloud-init-data/user-data
}


install_test_suite() {
    

    # 1. install docker
    # virt-customize -a /tmp/${GUEST_IMG} \
    #     --run ./cloud-init-data/init-scripts/script-test-suite-docker.sh

    # 2. download data
    # mkdir -p ./download
    # if [[ ! -f ./download/dien_bf16_pretrained_opt_model.pb ]]; then
    #     wget -P ./download https://storage.googleapis.com/intel-optimized-tensorflow/models/v2_5_0/dien_bf16_pretrained_opt_model.pb 
    # fi
    # if [[ ! -f ./download/dien_fp32_static_rnn_graph.pb ]]; then
    #     wget -P ./download https://storage.googleapis.com/intel-optimized-tensorflow/models/v2_5_0/dien_fp32_static_rnn_graph.pb 
    # fi
    
    # mkdir -p ./download/dien
    # if [[ ! -f ./download/data.tar.gz ]]; then
    #     wget -P ./download https://zenodo.org/record/3463683/files/data.tar.gz
    #     tar -C ./download/ -jxvf ./download/data.tar.gz
    #     mv ./download/data/* ./download/dien
    # fi

    # if [[ ! -f ./download/data1.tar.gz ]]; then
    #     wget -P ./download https://zenodo.org/record/3463683/files/data1.tar.gz
    #     tar -C ./download/ -jxvf ./download/data1.tar.gz
    #     mv ./download/data1/* ./download/dien
    # fi

    # if [[ ! -f ./download/data2.tar.gz ]]; then
    #     wget -P ./download https://zenodo.org/record/3463683/files/data2.tar.gz
    #     tar -C ./download/ -jxvf ./download/data2.tar.gz
    #     mv ./download/data2/* ./download/dien
    # fi
    
    # if [[ ! -d ./download/models ]]; then
    #     git clone https://github.com/IntelAI/models.git -b v2.5.0 ./download/models
    # fi
    
    # virt-customize -a /tmp/${GUEST_IMG} \
    #     --copy-in ./download/dien_bf16_pretrained_opt_model.pb:/root \
    #     --copy-in ./download/dien_fp32_static_rnn_graph.pb:/root \
    #     --copy-in ./download/dien:/root \
    #     --copy-in ./download/models:/root \
    # 3. download container image
    # virt-customize -a /tmp/${GUEST_IMG} \
    #     --run-command "sed -i 's/\[Service\]/\[Service\]\nEnvironment=\"HTTPS_PROXY=http:\/\/child-prc.intel.com:913\/\"/g' /usr/lib/systemd/system/docker.service" \
    #     --run-command "sed -i 's/\[Service\]/\[Service\]\nEnvironment=\"HTTP_PROXY=http:\/\/child-prc.intel.com:913\/\"/g' /usr/lib/systemd/system/docker.service" \
    #     --run-command "sed -i 's/\[Service\]/\[Service\]\nEnvironment=\"NO_PROXY=localhost,127.0.0.1,.intel.com\"/g' /usr/lib/systemd/system/docker.service"
    # enable_root_ssh_login
    # ../../../start-qemu.sh -i /tmp/${GUEST_IMG} -b grub -t legacy & 
    # sleep 10
    ssh -p 10026 root@localhost -o ConnectTimeout=30 "docker pull nginx:latest && docker pull redis:latest && intel/intel-optimized-tensorflow-avx512:2.8.0"
    
}

install_basic_packages() {
    # 1. install go 1.20.7
    if [[ ! -f ./download/go1.20.7.linux-amd64.tar.gz ]]; then
        wget -P ./download https://go.dev/dl/go1.20.7.linux-amd64.tar.gz
    fi
    virt-customize -a /tmp/${GUEST_IMG} \
        --copy-in ./download/go1.20.7.linux-amd64.tar.gz:/root
    virt-customize -a /tmp/${GUEST_IMG} \
        --run ./cloud-init-data/init-scripts/basic-install-go1.20.7.sh
}

configurate_vm() {
    virt-customize -a /tmp/${GUEST_IMG} \
        --run-command "sed -i 's/PasswordAuthentication no/PasswordAuthentication yes/g' /etc/ssh/sshd_config"
    if [[ $DEBUG_MODE == "true" ]]; then
        virt-customize -a /tmp/${GUEST_IMG} \
        --run-command "echo 'PermitRootLogin yes' >> /etc/ssh/sshd_config"
    fi
}

cleanup() {
    if [[ -f ${CURR_DIR}/"SHA256SUMS" ]]; then
        rm ${CURR_DIR}/"SHA256SUMS"
    fi
    ok "Cleanup!"
}

check_tool qemu-img
check_tool virt-customize
check_tool virt-install
check_tool genisoimage
check_tool cloud-init
check_tool git
check_tool awk
check_tool tree

process_args "$@"

#
# Check user permission
#
if (( $EUID != 0 )); then
    warn "Current user is not root, please use root permission via \"sudo\" or make sure current user has correct "\
         "permission by configuring /etc/libvirt/qemu.conf"
    warn "Please refer https://libvirt.org/drvqemu.html#posix-users-groups"
    sleep 5
fi


#==================== start ====================

# 1. basic image
create_guest_image
config_guest_env
resize_guest_image

# 2. create multi-part user-data
create_user_data

# 3. repo handle
prepare_repos

# 4. vm instance init
config_cloud_init

# 5. install packages
# 5.1 basic
install_basic_packages
install_tdx_measure_tool

# 5.2 test suite
# install_test_suite

# 6. config
configurate_vm

# 7. clean
cleanup

ok "Please get the output TDX guest image file at /tmp/${GUEST_IMG}"


#===========================objects

# 1. multi-imports & remote repo:
#   - url: option
#   - author: option
#   1.1 local
#   copy to image & apt source list & package install
#   1.2 remote
#   author (option) & apt source list & package install

# 2. template:
#   - pkgs lists:  multi-part
#   merge "cloud-init devel make-mime -a user-data-1:cloud-config -a user-data-2:cloud-config > user-data"
#   merge_how:
#  - name: list
#    settings: [append]
#  - name: dict
#    settings: [no_replace, recurse_list]

# 2.1 inner test 
#   + qemu-guest-agent host-repo
#   + docker install
#   + test data
#   + container image download: launch vm & ssh
#       - sshd config
#       - remote ssh run

# 3. secure
# 3.1. trim unnecessary packages
# 3.2. Immutable root filesystem
# 3.3. Stateless configuration
# 3.4. Security-hardened kernel
# 3.5. Security-centric defaults

# option/secure:
# 1. sshd config for debug
#       /etc/ssh/sshd_config: PermitRootLogin yes & PasswordAuthentication yes
# PasswordAuthentication no