#!/bin/bash

# Output file name
OUTPUT_FILE="${1:-docsdump_output.txt}"

# Maximum size for individual files to include (in bytes)
MAX_FILE_SIZE=5000000  # Skip files larger than ~5MB for docs

# Include patterns for documentation directories
INCLUDE_PATTERNS=(
    "*/docs/*"
    "*/documentation/*"
    "*/wiki/*"
    "*/man/*"
    "*/javadoc/*"
    "*/apidoc/*"
    "*/swagger/*"
    "*/manual/*"
    "*/dev-docs/*"
    # Add more documentation directories as needed
)

# Documentation file extensions to include
DOC_EXTENSIONS=(
    "md" "markdown" "rst" "adoc" "asciidoc" "txt" "wiki"
    "html" "htm" "rtf" "tex" "pdf" "djvu"
    "doc" "docx" "odt" "pages" "wpd"
    # Add more documentation extensions as needed
)

# Exclude patterns for non-documentation
EXCLUDE_PATTERNS=(
    # Common dependency dirs
    "*/node_modules/*"
    "*/target/*"
    "*/dist/*"
    "*/.git/*"
    "*/coverage/*"
    "*/.vscode/*"
    "*/build/*"
    
    # Binary files/directories
    "*/bin/*"
    
    # Generated files
    "*/generated/*"
    
    # Test directories
    "*/test/*"
    "*/tests/*"
    "*/__tests__/*"
    "*/spec/*"
)

# Check if a file is documentation based on extension
is_doc_extension() {
    local file_ext="$1"
    for ext in "${DOC_EXTENSIONS[@]}"; do
        if [[ "$file_ext" == "$ext" ]]; then
            return 0  # Is documentation
        fi
    done
    return 1  # Not documentation
}

# Check if a file path matches documentation directories
is_doc_directory() {
    local filepath="$1"
    for pattern in "${INCLUDE_PATTERNS[@]}"; do
        if [[ "$filepath" == $pattern ]]; then
            return 0  # Is in a documentation directory
        fi
    done
    return 1  # Not in a documentation directory
}

# Check if a file should be excluded
should_exclude() {
    local filepath="$1"
    for pattern in "${EXCLUDE_PATTERNS[@]}"; do
        if [[ "$filepath" == $pattern ]]; then
            return 0  # Should exclude
        fi
    done
    return 1  # Should not exclude
}

# Ensure required commands are available
for cmd in find cat wc file; do
    if ! command -v $cmd &> /dev/null; then
        echo "Error: $cmd command is required but not installed." >&2
        exit 1
    fi
done

echo "Generating documentation dump to $OUTPUT_FILE..."

# Generate header for the file
cat << EOF > "$OUTPUT_FILE"
# PROJECT DOCUMENTATION DUMP
# Generated on $(date)

## DOCUMENTATION STRUCTURE
\`\`\`
EOF

# Generate file tree of documentation files
find . -type f | grep -v "node_modules\|\.git\|target\|$OUTPUT_FILE" | sort | while read -r filepath; do
    # Get file extension
    ext="${filepath##*.}"
    
    # Include only documentation files
    if is_doc_extension "$ext" || is_doc_directory "$filepath"; then
        echo "$filepath" >> "$OUTPUT_FILE"
    fi
done

# Close the code block
echo -e "\`\`\`\n\n## DOCUMENTATION CONTENTS\n" >> "$OUTPUT_FILE"

# Initialize counters in a temporary file
echo "0" > /tmp/included_docs.txt
echo "0" > /tmp/total_files.txt
echo "0" > /tmp/skipped_large.txt
echo "0" > /tmp/skipped_binary.txt
echo "0" > /tmp/skipped_non_docs.txt

# Find all files, filter for documentation
find . -type f | grep -v "node_modules\|\.git\|target\|$OUTPUT_FILE" | sort | while read -r filepath; do
    # Increment total files counter
    curr_total=$(<"/tmp/total_files.txt")
    echo $((curr_total + 1)) > /tmp/total_files.txt
    
    # Get file extension
    ext="${filepath##*.}"
    
    # Include only documentation files
    is_doc=0
    if is_doc_extension "$ext" || is_doc_directory "$filepath"; then
        is_doc=1
    else
        curr_skipped=$(<"/tmp/skipped_non_docs.txt")
        echo $((curr_skipped + 1)) > /tmp/skipped_non_docs.txt
        continue
    fi
    
    # Skip files that should be excluded
    if should_exclude "$filepath"; then
        curr_skipped=$(<"/tmp/skipped_non_docs.txt")
        echo $((curr_skipped + 1)) > /tmp/skipped_non_docs.txt
        continue
    fi
    
    # Skip files that are too large
    filesize=$(wc -c < "$filepath" 2>/dev/null || echo 0)
    if [ "$filesize" -gt "$MAX_FILE_SIZE" ]; then
        curr_skipped=$(<"/tmp/skipped_large.txt")
        echo $((curr_skipped + 1)) > /tmp/skipped_large.txt
        continue
    fi
    
    # Skip binary documentation files (like PDFs) that can't be displayed as text
    if [[ "$ext" != "md" && "$ext" != "markdown" && "$ext" != "rst" && "$ext" != "adoc" && "$ext" != "asciidoc" && "$ext" != "txt" && "$ext" != "wiki" && "$ext" != "html" && "$ext" != "htm" ]]; then
        if file "$filepath" | grep -q "binary\|data\|executable"; then
            curr_skipped=$(<"/tmp/skipped_binary.txt")
            echo $((curr_skipped + 1)) > /tmp/skipped_binary.txt
            continue
        fi
    fi
    
    # Add file to output
    echo -e "### FILE: $filepath" >> "$OUTPUT_FILE"
    echo -e "\`\`\`$ext" >> "$OUTPUT_FILE"
    cat "$filepath" 2>/dev/null >> "$OUTPUT_FILE" || echo "Unable to read file" >> "$OUTPUT_FILE"
    echo -e "\`\`\`\n" >> "$OUTPUT_FILE"
    
    # Increment included files counter
    curr_included=$(<"/tmp/included_docs.txt")
    echo $((curr_included + 1)) > /tmp/included_docs.txt
done

# Read counter values
INCLUDED_DOCS=$(<"/tmp/included_docs.txt")
TOTAL_FILES=$(<"/tmp/total_files.txt")
SKIPPED_LARGE=$(<"/tmp/skipped_large.txt")
SKIPPED_BINARY=$(<"/tmp/skipped_binary.txt")
SKIPPED_NON_DOCS=$(<"/tmp/skipped_non_docs.txt")

# Clean up temp files
rm -f /tmp/included_docs.txt /tmp/total_files.txt /tmp/skipped_large.txt /tmp/skipped_binary.txt /tmp/skipped_non_docs.txt

# Add summary information
echo -e "\n## SUMMARY\n" >> "$OUTPUT_FILE"
echo "Total files scanned: $TOTAL_FILES" >> "$OUTPUT_FILE"
echo "Documentation files included: $INCLUDED_DOCS" >> "$OUTPUT_FILE"
echo "Files skipped due to size: $SKIPPED_LARGE" >> "$OUTPUT_FILE"
echo "Binary documentation files skipped: $SKIPPED_BINARY" >> "$OUTPUT_FILE"
echo "Non-documentation files skipped: $SKIPPED_NON_DOCS" >> "$OUTPUT_FILE"

echo "Documentation dump completed. Output saved to $OUTPUT_FILE"
echo "Included $INCLUDED_DOCS documentation files out of $TOTAL_FILES total files"