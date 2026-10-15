function Write-ReleaseZip([string]$Stage, [string]$Destination) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $root = (Get-Item -LiteralPath $Stage).FullName.TrimEnd('\','/')
    $prefix = (Split-Path $root -Leaf) + '/'
    $zip = [IO.Compression.ZipFile]::Open($Destination, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($file in Get-ChildItem -LiteralPath $root -Recurse -File) {
            # Windows PowerShell's Compress-Archive can emit backslashes, which aren't ZIP separators.
            $entry = $prefix + $file.FullName.Substring($root.Length + 1).Replace('\','/')
            [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $file.FullName, $entry, [IO.Compression.CompressionLevel]::Optimal)
        }
    } finally { $zip.Dispose() }
}
