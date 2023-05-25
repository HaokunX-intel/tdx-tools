.PHONY: all clean deps

all: deps
	cargo build --release
	strip --strip-all target/release/fde-agent
	
clean:
	cargo clean
	make clean -C SGXDataCenterAttestationPrimitives/QuoteGeneration/quote_wrapper/qgs_msg_lib/linux
	make clean -C SGXDataCenterAttestationPrimitives/QuoteGeneration/quote_wrapper/tdx_attest/linux	
deps:
	make -C SGXDataCenterAttestationPrimitives/QuoteGeneration/quote_wrapper/qgs_msg_lib/linux
	make -C SGXDataCenterAttestationPrimitives/QuoteGeneration/quote_wrapper/tdx_attest/linux	
	