CL_CASPER_NUM_VALIDATORS ?= 3
CL_SERVER_HTTP_PORT ?= 40403
CL_VERSION ?= latest

REFRESH_TARGETS   := ./monitoring/prometheus/refresh-targets.sh
HIGHWAY_ENV       := .casperlabs/chainspec/genesis/highway.env
RESET_HIGHWAY_ENV := rm -f $(HIGHWAY_ENV); ./scripts/highway-env.sh > $(HIGHWAY_ENV)

# Don't remove directories created as dependencies.
.SECONDARY:

# Remove all node-N environments.
clean: down $(shell find . -type d -name "node-*" | grep -v .casperlabs | grep -v keys | awk -F '/' '{print $$2"/down"}')
	docker network rm casperlabs || exit 0
	rm -rf .casperlabs
	rm -rf .make
	rm -rf keys

# Create a new node environment by linking to the template docker-complse.yml file.
node-%: .casperlabs
	$(eval N = $*)
	$(eval ENV = node-$(N)/.env)
	mkdir node-$(N)

	@# Create an .env file to hold template variables for docker-compose.
	echo NODE_NUMBER=$(N) >> $(ENV)

	# Set default env values. Create overrides with `export CL_<name>=<value>` before running `make node-X/up` commands,
	# or override them for individual nodes e.g. `CL_VERSION=0.1.2 make node-0/up`
	# Capture all CL_ env vars into the .env file so later they don't revert if we run things in another console.
	# Take overrides from template/.env file.
	bash -c '\
	export CL_VERSION=$(CL_VERSION) ; \
	while read kv; do \
		k=$$(echo $${kv} | cut -d'=' -f1) ; \
		v=$$(echo $${kv} | cut -d'=' -f2) ; \
		if [ -z "$${!k}" ]; then \
			export $${k}=$${v} ; \
		fi ; \
	done < ${PWD}/template/.env ; \
	for var in $$(compgen -v CL_); do \
		echo $${var}=$${!var} >> $(ENV) ; \
	done'

	@# Link Dockerfile so we can change it and reapply.
	@#ln -s ${PWD}/template/docker-compose.yml node-$(N)/docker-compose.yml
	@#Alternatively just make a copy so you can edit it independently.
	cp ${PWD}/template/docker-compose.yml node-$(N)/docker-compose.yml


# Start node.
# Only make `highway.env` when the first node is started, so the era epoch doesn't become stale.
node-%/up: node-% .make/docker/network maybe-reset-highway-env
	@# Go into the directory to pick up values from .env
	if [ "$(CL_VERSION)" != "latest" ]; then \
		cd node-$* && docker-compose pull; \
	fi
	cd node-$* && docker-compose up -d
	$(REFRESH_TARGETS)

# Tear down node. Using docker to delete logs owned by root for now.
node-%/down:
	if [ -d node-$* ]; then \
		cd node-$* && docker-compose down && cd - && \
		docker run -it --rm \
			-v $(PWD)/node-$*/logs:/var/logs/casperlabs \
			--entrypoint sh casperlabs/node:$(CL_VERSION) \
			-c 'rm -rf /var/logs/casperlabs/*' && \
		rm -rf node-$* ; \
	fi
	$(REFRESH_TARGETS)

# Just a quick way to check whether the server published any Prometheus metrics yet.
node-%/metrics:
	docker run --rm --network casperlabs appropriate/curl -s http://node-$*:$(CL_SERVER_HTTP_PORT)/metrics


# Start common components.
up: .make/docker/network .casperlabs
	$(REFRESH_TARGETS)
	if [ "$(CL_VERSION)" != "latest" ]; then \
		docker-compose pull; \
	fi
	docker-compose -p casperlabs up -d --remove-orphans

# Stop common components.
down:
	docker-compose -p casperlabs down


# Slow down traffic between nodes to simulate network effects and large blocks.
# If we slow it down too much then the client won't be able to connect though.
# https://alexei-led.github.io/post/pumba_docker_netem/
delay:
	docker run -it --rm -v /var/run/docker.sock:/var/run/docker.sock gaiaadm/pumba \
	    netem \
	      --duration 5m \
	      --interface eth0 \
	      --tc-image gaiadocker/iproute2 \
	      delay \
	        --time 500 \
	        --jitter 100 \
	        --distribution normal \
	      re2:^node

# Use the `rate` function to limit bandwidth.
slow:
	docker run -it --rm -v /var/run/docker.sock:/var/run/docker.sock gaiaadm/pumba \
	    netem \
	      --duration 5m \
	      --interface eth0 \
	      --tc-image gaiadocker/iproute2 \
	      rate \
	        --rate 100kbit \
	      re2:^node


.make/docker/network:
	docker network create casperlabs
	mkdir -p $(dir $@) && touch $@

# Generate keys and bonds.
.casperlabs:
	mkdir -p .casperlabs/bootstrap
	mkdir -p .casperlabs/chainspec/genesis

	@# Create a `facet-account` to hold some initial tokens to distribute.
	mkdir -p keys/faucet-account
	../key-management/docker-gen-account-keys.sh keys/faucet-account
	(cat keys/faucet-account/account-id; echo ",10000000000,0") > .casperlabs/chainspec/genesis/accounts.csv

	@# Create bonded validators with 0 balance.
	bash -c 'i=0 ; while [[ $$i -lt $(CL_CASPER_NUM_VALIDATORS) ]] ; do \
		echo Generating validator $$i / $(CL_CASPER_NUM_VALIDATORS) ; \
		mkdir -p .casperlabs/node-$$i ; \
		mkdir -p keys/account-$$i ; \
		../key-management/docker-gen-keys.sh .casperlabs/node-$$i ; \
		../key-management/docker-gen-account-keys.sh keys/account-$$i ; \
		BOND=$$(( $(CL_CASPER_NUM_VALIDATORS)*10+$$i )) ; \
		(cat .casperlabs/node-$$i/validator-id; echo ",0,$$BOND") >> .casperlabs/chainspec/genesis/accounts.csv ; \
		((i = i + 1)) ; \
	done'

	@# Copy the bootstrap node to a place where every node can read it from.
	cp -r .casperlabs/node-0/node-id .casperlabs/bootstrap/node-id

	@# Check that the files we wanted exist and aren't empty.
	[ -s .casperlabs/chainspec/genesis/accounts.csv ]
	[ -s .casperlabs/bootstrap/node-id ]


# Create common Highway environment overrides for the chainspec defaults,
# common to all, so a node started later won't choose a different era epoch.
$(HIGHWAY_ENV): reset-highway-env

# If we need to re-start the nodes after a long time, we might have to recreate the era epoch, if it's short, like 5 minutes.
reset-highway-env: .casperlabs
	$(RESET_HIGHWAY_ENV)

# Reset highway env if there are no nodes running at the moment or there's only one node directory created (we are restarting the 1 node we have)
maybe-reset-highway-env:
	DEFINED=$$(find . -maxdepth 1 -type d | grep -e 'node-[0-9]' | wc -l); \
	RUNNING=$$(docker ps --format '{{.Names}}' | grep -e 'node-[0-9]' | wc -l); \
	if [ "$${RUNNING}" = "0" ] || [ "$${DEFINED}" = "1" ]; then \
		$(RESET_HIGHWAY_ENV); \
	fi
