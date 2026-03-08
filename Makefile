.PHONY: bump

bump:
	@current=$$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/'); \
	major=$$(echo $$current | cut -d. -f1); \
	minor=$$(echo $$current | cut -d. -f2); \
	patch=$$(echo $$current | cut -d. -f3); \
	new="$$major.$$minor.$$((patch + 1))"; \
	sed -i '' "s/^version = \".*\"/version = \"$$new\"/" Cargo.toml; \
	echo "Bumped version $$current → $$new"; \
	git add Cargo.toml; \
	git commit -m "bump version to $$new"; \
	git tag $$new; \
	git push --tags
