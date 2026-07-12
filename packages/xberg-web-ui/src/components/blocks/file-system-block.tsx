import { FileSystem, type FileSystemProps } from "@/components/ui/file-system"

export type FileSystemBlockProps = FileSystemProps

// The PDF, DOCX, XLSX, and image viewer dialogs are built into FileSystem;
// the block exists as a composed, installable example of the full browser.
export function FileSystemBlock(props: FileSystemBlockProps) {
  return <FileSystem {...props} />
}
