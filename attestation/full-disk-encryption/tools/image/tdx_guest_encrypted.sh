#!/bin/bash

set -ex

THIS_DIR=$(dirname "$(readlink -f "$0")")

KEY=${THIS_DIR}/key
IMG_URL=https://cloud-images.ubuntu.com/jammy/current
CLOUD_IMG=jammy-server-cloudimg-amd64.img
TD_IMG=td-guest-ubuntu-22.04.qcow2
TMP_IMG=ubuntu-22.04.qcow2
TDX_REPO_URL="/srv/guest_repo"
ROOT_PASS="123456"

usage() {
    cat << EOM
Usage: $(basename "$0") [OPTION]...
  -p <guest root password>          Default is 123456, recommend changing it
  -k <disk encryption key file>     Default is key under current directory
  -u <guest TDX package repo>       Default is local repo /srv/guest_repo
  -h                                Show this help
EOM
}

process_args() {
    while getopts ":k:p:u:" option; do
        case "$option" in
            p) ROOT_PASS=$OPTARG;;
            k) KEY=$OPTARG;;
            u) TDX_REPO_URL=$OPTARG;;
            h) usage
               exit 0
               ;;
            *)
               echo "Invalid option '-$OPTARG'"
               usage
               exit 1
               ;;
        esac
    done
}

process_args "$@"

if ! command -v "virt-customize" ; then
    echo "virt-customize not found, please install libguestfs-tools"
    exit 1
fi

# Check the nbd kernel module
if ! lsmod | grep nbd ; then
    echo "nbd module not found, please try modprobe -av nbd"
    exit 1
fi

# Download cloud image
if [[ ! -f ${CLOUD_IMG} ]] ; then
    wget ${IMG_URL}/${CLOUD_IMG}
fi

# The original image is in qcow2 format already.
cp ${CLOUD_IMG} ${TD_IMG}
cp ${CLOUD_IMG} ${TMP_IMG}

# Set repo list
REPO_LIST="deb [trusted=yes] ${TDX_REPO_URL} jammy/amd64/"

# Setup guest environments
virt-customize -a ${TMP_IMG}  --root-password password:${ROOT_PASS}
ARGS=" -a ${TMP_IMG} -v"
ARGS+=" --copy-in /etc/environment:/etc"
ARGS+=" --copy-in netplan.yaml:/etc/netplan/"
ARGS+=" --edit '/etc/ssh/sshd_config:s/#PermitRootLogin prohibit-password/PermitRootLogin yes/'"
ARGS+=" --edit '/etc/ssh/sshd_config:s/PasswordAuthentication no/PasswordAuthentication yes/'"
ARGS+=" --run-command 'ssh-keygen -A'"
ARGS+=" --run-command 'systemctl mask pollinate.service'"
ARGS+=" --run-command 'echo ${REPO_LIST} | tee /etc/apt/sources.list.d/intel-tdx.list'"
ARGS+=" --run-command 'apt update && apt install -y linux-image-unsigned-6.2.0-mvp*'"
echo "${ARGS}"
eval virt-customize "${ARGS}"

# Expand 1G for boot partition
qemu-img resize ${TD_IMG} +1G

# Attach QEMU image to devices
qemu-nbd -c /dev/nbd0 ${TMP_IMG}
qemu-nbd -c /dev/nbd1 ${TD_IMG}

# Add boot partition 16 before rootfs partition 1
echo -e 'd\n1\nn\n16\n\n+1G\nn\n1\n\n\nwq\n'  | fdisk /dev/nbd1
# Format boot partition
mkfs.ext4 -F /dev/nbd1p16

# Mount 2 disk boot partitions
mkdir /mnt/nbd0
mkdir /mnt/nbd1
mount /dev/nbd0p1 /mnt/nbd0/
mount /dev/nbd1p16 /mnt/nbd1/
# Copy boot data from another disk
cp -ar /mnt/nbd0/boot/* /mnt/nbd1/
e2label /dev/nbd1p16 boot
# Umount boot partitions
umount /mnt/nbd1/
umount /mnt/nbd0/

# Setup partition to LUKS with cipher aes-gcm-random
cryptsetup -v -q luksFormat --encrypt --type luks2 --key-file ${KEY} \
  --cipher aes-gcm-random --integrity aead --key-size 256 /dev/nbd1p1
cryptsetup -v config --label cloudimg-rootfs-enc /dev/nbd1p1

# Open the partition and dd rootfs to it
cryptsetup open --key-size 256 --key-file ${KEY} /dev/nbd1p1 nbd1p1
dd if=/dev/nbd0p1 of=/dev/mapper/nbd1p1 bs=512
e2fsck -fy /dev/mapper/nbd1p1

# Copy initramfs tools and FDE agent to root partition
mount /dev/mapper/nbd1p1 /mnt/nbd1/
cp -arT ../initramfs-tools/* /mnt/nbd1/usr/share/initramfs-tools/
cp ../../target/release/fde-agent /mnt/nbd1/sbin/
umount /mnt/nbd1/

# Clean up environment
cryptsetup close nbd1p1
qemu-nbd -d /dev/nbd0
qemu-nbd -d /dev/nbd1
