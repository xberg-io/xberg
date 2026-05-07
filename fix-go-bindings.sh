#!/bin/bash
# Post-generation fix for Go bindings
# Maps unresolved office metadata struct types to json.RawMessage
# This is a workaround until alef resolves cross-module struct types correctly

set -euo pipefail

BINDING_FILE="packages/go/v5/binding.go"

if [ ! -f "$BINDING_FILE" ]; then
  echo "Error: $BINDING_FILE not found"
  exit 1
fi

# Step 1: Map struct field types to json.RawMessage
replacements=(
  # DocxMetadata fields
  "s/CoreProperties \*string/CoreProperties json.RawMessage/g"
  "s/AppProperties \*string/AppProperties json.RawMessage/g"
  "s/CustomProperties \*string/CustomProperties json.RawMessage/g"
  # OdtMetadata fields
  "s/OdtMetaProperties \*string/OdtMetaProperties json.RawMessage/g"
  # PptxMetadata fields
  "s/PptxAppProperties \*string/PptxAppProperties json.RawMessage/g"
  # XlsxMetadata fields
  "s/XlsxAppProperties \*string/XlsxAppProperties json.RawMessage/g"
)

for replacement in "${replacements[@]}"; do
  sed -i "" "$replacement" "$BINDING_FILE"
done

# Step 2: Fix builder functions to accept json.RawMessage
# Change from: func WithDocxMetadataCoreProperties(v string) DocxMetadataOption
# Change to:   func WithDocxMetadataCoreProperties(v json.RawMessage) DocxMetadataOption

builder_fixes=(
  "s/func (WithDocxMetadataCoreProperties)(v string)/func \1(v json.RawMessage)/g"
  "s/func (WithDocxMetadataAppProperties)(v string)/func \1(v json.RawMessage)/g"
  "s/func (WithDocxMetadataCustomProperties)(v string)/func \1(v json.RawMessage)/g"
  "s/func (WithOdtMetadataOdtMetaProperties)(v string)/func \1(v json.RawMessage)/g"
  "s/func (WithPptxMetadataPptxAppProperties)(v string)/func \1(v json.RawMessage)/g"
  "s/func (WithXlsxMetadataXlsxAppProperties)(v string)/func \1(v json.RawMessage)/g"
)

for fix in "${builder_fixes[@]}"; do
  sed -i "" "$fix" "$BINDING_FILE"
done

# Step 3: Fix assignments in builder functions
# Change from: return func(c *DocxMetadata) { c.CoreProperties = &v }
# Change to:   return func(c *DocxMetadata) { c.CoreProperties = v }

assignment_fixes=(
  "s/c\.CoreProperties = &v/c.CoreProperties = v/g"
  "s/c\.AppProperties = &v/c.AppProperties = v/g"
  "s/c\.CustomProperties = &v/c.CustomProperties = v/g"
  "s/c\.OdtMetaProperties = &v/c.OdtMetaProperties = v/g"
  "s/c\.PptxAppProperties = &v/c.PptxAppProperties = v/g"
  "s/c\.XlsxAppProperties = &v/c.XlsxAppProperties = v/g"
)

for fix in "${assignment_fixes[@]}"; do
  sed -i "" "$fix" "$BINDING_FILE"
done

echo "Fixed Go bindings: mapped office metadata types to json.RawMessage"
