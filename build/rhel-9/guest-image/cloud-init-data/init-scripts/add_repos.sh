#!/bin/bash

# cat <<EOT >> /etc/yum.repos.d/rocky.repo
# [rocky-appstream]
# name = rocky-appstream 
# baseurl = https://download.rockylinux.org/pub/rocky/9/AppStream/x86_64/os/ 
# enabled = 1
# gpgcheck = 0
# EOT

dnf check-update

