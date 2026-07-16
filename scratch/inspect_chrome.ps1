Get-Process | Where-Object { $_.MainWindowHandle -ne 0 } | ForEach-Object {
    [PSCustomObject]@{
        Id = $_.Id
        Name = $_.Name
        Title = $_.MainWindowTitle
        Handle = $_.MainWindowHandle
    }
} | Format-Table
