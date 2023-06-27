# Test scripts

define unit_test
  cargo test -p percpu $(1) -- --nocapture
  cargo test -p axfs $(1) --features "myfs" -- --nocapture
  cargo test --workspace --exclude "arceos-*" --exclude "std-*" --exclude "arceos_api" $(1) -- --nocapture
endef

define app_test
  $(CURDIR)/scripts/test/app_test.sh
endef

define stdapp_test
  $(CURDIR)/scripts/test/stdapp_test.sh
endef
