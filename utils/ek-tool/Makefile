######## SGX SDK Settings ########

SGX_SDK ?= /opt/intel/sgxsdk
SGX_ARCH ?= x64

# Don't support 32bit in this sample
SGX_COMMON_FLAGS := -m64
SGX_LIBRARY_PATH := $(SGX_SDK)/lib64
SGX_ENCLAVE_SIGNER := $(SGX_SDK)/bin/x64/sgx_sign
SGX_EDGER8R := $(SGX_SDK)/bin/x64/sgx_edger8r


SGX_COMMON_FLAGS += -O2
SGX_COMMON_FLAGS += -DSGX_TRUSTED -Wall -Wextra -Winit-self -Wpointer-arith -Wreturn-type \
                    -Waddress -Wsequence-point -Wformat-security \
                    -Wmissing-include-dirs -Wfloat-equal -Wundef -Wshadow \
                    -Wcast-align -Wcast-qual -Wconversion -Wredundant-decls
SGX_COMMON_CFLAGS := $(SGX_COMMON_FLAGS) -Wjump-misses-init -Wstrict-prototypes -Wunsuffixed-float-constants
SGX_COMMON_CXXFLAGS := $(SGX_COMMON_FLAGS) -Wnon-virtual-dtor -std=c++11

######## Lib Settings ########

Urts_Library_Name := sgx_urts

Lib_Cpp_Files := Lib/Lib.cpp

Lib_Include_Paths := -ILib -I$(SGX_SDK)/include

Lib_C_Flags := -fPIC -Wno-attributes $(Lib_Include_Paths)
Lib_C_Flags += -DNDEBUG -UEDEBUG -UDEBUG

Lib_Cpp_Flags := $(Lib_C_Flags)
Lib_Link_Flags := -L$(SGX_LIBRARY_PATH) -lsgx_dcap_quoteverify -l$(Urts_Library_Name) -lpthread -ldl

Lib_Link_Flags += -ldcap_quoteprov
Lib_Cpp_Flags += -DSGX_QPL_LOGGING=SGX_QL_LOG_ERROR

Lib_Cpp_Objects := $(Lib_Cpp_Files:.cpp=.o)

App_Name := ek-tool

Lib_Name := libenclave_untrusted.a

Build_Mode = HW_RELEASE

.PHONY: all target

all: .config_$(Build_Mode)_$(SGX_ARCH)
	@$(MAKE) target

target:  $(App_Name)
	@echo "The project has been built in release hardware mode."

.config_$(Build_Mode)_$(SGX_ARCH):
	@rm -f .config_* $(Lib_Cpp_Objects) Lib/Enclave_u.* $(Enclave_Cpp_Objects) Enclave/Enclave_t.*
	@touch .config_$(Build_Mode)_$(SGX_ARCH)

######## Lib Objects ########

Lib/Enclave_u.h: $(SGX_EDGER8R) Enclave/Enclave.edl
	@cd Lib && $(SGX_EDGER8R) --untrusted ../Enclave/Enclave.edl --search-path ../Enclave --search-path $(SGX_SDK)/include
	@echo "GEN  =>  $@"

Lib/Enclave_u.c: Lib/Enclave_u.h

Lib/Enclave_u.o: Lib/Enclave_u.c
	@$(CC) $(SGX_COMMON_CFLAGS) $(Lib_C_Flags) -c $< -o $@
	@echo "CC   <=  $<"

$(App_Name): $(Lib_Name)
	@cargo build --release

$(Lib_Name): Lib/Enclave_u.o
	@ar rs Lib/$@ $^
	@echo "AR => Lib/$@"

.PHONY: clean

clean:
	@cargo clean
	@rm -f .config_* $(Lib_Cpp_Objects) Lib/*_u.* $(Enclave_Cpp_Objects) Enclave/Enclave_t.* Lib/$(Lib_Name)
