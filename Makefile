.PHONY: build test fmt clippy run doctor install-plan docs clean

REF := docs/templates/reference_tnr_fangsong_redcode.docx

build:
	cargo build --workspace -j 6

test:
	cargo test --workspace

fmt:
	cargo fmt --all --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

run: build
	cargo run -p connexon-cli -- run examples/hello.cnx

doctor: build
	cargo run -p connexon-cli -- doctor

install-plan: build
	cargo run -p connexon-cli -- install --plan-only

docs:
	pandoc README.md --reference-doc=$(REF) -o README.docx
	pandoc AUTHORS.md --reference-doc=$(REF) -o AUTHORS.docx
	pandoc docs/TUTORIAL.md --reference-doc=$(REF) -o docs/TUTORIAL.docx
	pandoc docs/API_REFERENCE.md --reference-doc=$(REF) -o docs/API_REFERENCE.docx
	pandoc docs/DEVELOPER_GUIDE.md --reference-doc=$(REF) -o docs/DEVELOPER_GUIDE.docx
	pandoc docs/FAQ.md --reference-doc=$(REF) -o docs/FAQ.docx
	pandoc docs/LANGUAGE.md --reference-doc=$(REF) -o docs/LANGUAGE.docx
	pandoc docs/DESIGN.md --reference-doc=$(REF) -o docs/DESIGN.docx
	pandoc docs/ROADMAP.md --reference-doc=$(REF) -o docs/ROADMAP.docx
	pandoc handoff.md --reference-doc=$(REF) -o handoff.docx

clean:
	cargo clean
	rm -f connexon.install-plan.json
