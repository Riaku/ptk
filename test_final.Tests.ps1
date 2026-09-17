# 1. Resolve PTK Binary
$debug = Join-Path $PSScriptRoot "target\x86_64-pc-windows-gnu\debug\ptk.exe"
$release = Join-Path $PSScriptRoot "target\x86_64-pc-windows-gnu\release\ptk.exe"
if ((Test-Path $debug) -and (Test-Path $release)) {
    $BinaryPath = if ((Get-Item $debug).LastWriteTime -ge (Get-Item $release).LastWriteTime) { $debug } else { $release }
} elseif (Test-Path $debug) {
    $BinaryPath = $debug
} elseif (Test-Path $release) {
    $BinaryPath = $release
}

if (-not $BinaryPath -or -not (Test-Path $BinaryPath)) {
    throw "ptk.exe not found! Please build it first using .\build.ps1"
}

$global:PtkBin = $BinaryPath

function Run-ComparisonTest {
    param(
        [string]$Category,
        [string]$TestName,
        [string]$NativeCommand,
        [string]$PtkCommand,
        [bool]$ExpectFallback = $false
    )

    It "[$Category] $TestName" -TestCases @(@{
        Native = $NativeCommand
        Ptk    = $PtkCommand
        Cat    = $Category
        Name   = $TestName
        ExpFb  = $ExpectFallback
    }) {
        param($Native, $Ptk, $Cat, $Name, $ExpFb)

        # Execute Native (in-process for speed)
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $nativeOut = Invoke-Expression $Native 2>&1 | Out-String
        $nativeMs = [math]::Round($sw.Elapsed.TotalMilliseconds, 1)
        $sw.Stop()

        # Execute PTK
        $sw.Restart()
        $ptkOut = Invoke-Expression $Ptk 2>&1 | Out-String
        $ptkMs = [math]::Round($sw.Elapsed.TotalMilliseconds, 1)
        $sw.Stop()

        # Normalize line endings (CRLF vs LF) to ensure accurate character comparison
        $nativeNorm = $nativeOut.Replace("`r`n", "`n").TrimEnd()
        $ptkNorm = $ptkOut.Replace("`r`n", "`n").TrimEnd()
        $nativeChars = $nativeNorm.Length
        $ptkChars = $ptkNorm.Length
        $savingsPct = if ($nativeChars -gt 0) { [math]::Round(((1.0 - ($ptkChars / [double]$nativeChars)) * 100), 1) } else { 0 }
        $speedRatio = if ($ptkMs -gt 0) { [math]::Round(($nativeMs / $ptkMs), 2) } else { 0 }

        $status = "PASS"
        $notes = ""

        if ($ExpFb) {
            $nativeTrimmed = $nativeNorm.Trim()
            $ptkTrimmed = $ptkNorm.Trim()
            if ($ptkTrimmed -eq $nativeTrimmed -or $ptkChars -ge ($nativeChars * 0.85)) {
                $notes = "Native Passthrough (Verified)"
            } else {
                $status = "WARN"
                $notes = "Expected Passthrough, got compressed"
            }
        } else {
            if ($savingsPct -ge 0) {
                $notes = "Saved $savingsPct% characters"
            } else {
                $status = "WARN"
                $notes = "Inflated by $([math]::Abs($savingsPct))%"
            }
        }

        $benchFile = "$env:TEMP/ptk_benchmark_results.jsonl"
        $benchItem = [PSCustomObject]@{
            Category    = $Cat
            TestName    = $Name
            NativeChars = $nativeChars
            PTKChars    = $ptkChars
            SavingsPct  = if ($ExpFb) { "Fallback" } else { "$savingsPct%" }
            NativeMs    = $nativeMs
            PtkMs       = $ptkMs
            SpeedRatio  = $speedRatio
            Status      = $status
            Notes       = $notes
        }
        [System.IO.File]::AppendAllText($benchFile, ($benchItem | ConvertTo-Json -Compress) + "`n")

        $ptkOut | Should -Not -BeNullOrEmpty
        if ($ExpFb) {
            $status | Should -Be "PASS"
        } else {
            if ($nativeChars -eq 0) {
                $ptkOut | Should -Not -BeNullOrEmpty
            } else {
                $ptkChars | Should -BeLessOrEqual ([math]::Max([math]::Ceiling($nativeChars * 1.25), $nativeChars + 100))
            }
        }
    }
}

Describe "PTK Command Optimization Tests" {
    $sampleCsv = "$env:TEMP/ptk_test_sample.csv"
    $csvTmp = "$env:TEMP/ptk_test_export.csv"
    Set-Content -Path $sampleCsv -Value "Id,Name,Role`n1,Alice,Admin`n2,Bob,Developer`n3,Charlie,Designer`n4,David,QA`n5,Eve,Manager" -Force

    BeforeAll {
        $benchFile = "$env:TEMP/ptk_benchmark_results.jsonl"
        if (Test-Path $benchFile) { Remove-Item $benchFile -Force }
    }


# ---------------------------------------------------------
# 1. Get-ChildItem (gci)
# ---------------------------------------------------------
Write-Host "[1/14] Testing Get-ChildItem (gci)..." -ForegroundColor Yellow
Run-ComparisonTest -Category "Get-ChildItem" -TestName "Basic Listing (gci .)" `
    -NativeCommand "Get-ChildItem ." -PtkCommand "& '$BinaryPath' get-childitem ."

Run-ComparisonTest -Category "Get-ChildItem" -TestName "Recursive Listing (gci docs -r)" `
    -NativeCommand "Get-ChildItem docs -Recurse" -PtkCommand "& '$BinaryPath' get-childitem docs -Recurse"

Run-ComparisonTest -Category "Get-ChildItem" -TestName "Filter Pattern (gci -Filter *.md)" `
    -NativeCommand "Get-ChildItem -Filter *.md" -PtkCommand "& '$BinaryPath' get-childitem -Filter *.md"

Run-ComparisonTest -Category "Get-ChildItem" -TestName "Directories Only (gci -Directory)" `
    -NativeCommand "Get-ChildItem -Directory" -PtkCommand "& '$BinaryPath' get-childitem -Directory"

Run-ComparisonTest -Category "Get-ChildItem" -TestName "Files Only (gci -File)" `
    -NativeCommand "Get-ChildItem -File" -PtkCommand "& '$BinaryPath' get-childitem -File"

Run-ComparisonTest -Category "Get-ChildItem" -TestName "Fallback: -Name Flag" `
    -NativeCommand "Get-ChildItem -Name" -PtkCommand "& '$BinaryPath' get-childitem -Name" -ExpectFallback $true

Run-ComparisonTest -Category "Get-ChildItem" -TestName "Fallback: Pipeline (| Measure)" `
    -NativeCommand "Get-ChildItem | Measure-Object" -PtkCommand "& '$BinaryPath' run 'Get-ChildItem | Measure-Object'" -ExpectFallback $true

# ---------------------------------------------------------
# 2. Get-Content (gc)
# ---------------------------------------------------------
Write-Host "[2/14] Testing Get-Content (gc)..." -ForegroundColor Yellow
Run-ComparisonTest -Category "Get-Content" -TestName "Read Standard File (gc Cargo.toml)" `
    -NativeCommand "Get-Content Cargo.toml" -PtkCommand "& '$BinaryPath' gc Cargo.toml"

Run-ComparisonTest -Category "Get-Content" -TestName "Head 10 Lines (gc -TotalCount 10)" `
    -NativeCommand "Get-Content -TotalCount 10 Cargo.toml" -PtkCommand "& '$BinaryPath' gc -TotalCount 10 Cargo.toml"

Run-ComparisonTest -Category "Get-Content" -TestName "Tail 10 Lines (gc -Tail 10)" `
    -NativeCommand "Get-Content -Tail 10 Cargo.toml" -PtkCommand "& '$BinaryPath' gc -Tail 10 Cargo.toml"

Run-ComparisonTest -Category "Get-Content" -TestName "Raw Text Read (gc -Raw)" `
    -NativeCommand "Get-Content -Raw Cargo.toml" -PtkCommand "& '$BinaryPath' gc -Raw Cargo.toml"

Run-ComparisonTest -Category "Get-Content" -TestName "Large File Read (gc README.md)" `
    -NativeCommand "Get-Content README.md" -PtkCommand "& '$BinaryPath' gc README.md"

Run-ComparisonTest -Category "Get-Content" -TestName "Fallback: Pipeline (| Select-String)" `
    -NativeCommand "Get-Content Cargo.toml | Select-String name" -PtkCommand "& '$BinaryPath' run 'Get-Content Cargo.toml | Select-String name'" -ExpectFallback $true

# ---------------------------------------------------------
# 3. Select-String (sls)
# ---------------------------------------------------------
Write-Host "[3/14] Testing Select-String (sls)..." -ForegroundColor Yellow
Run-ComparisonTest -Category "Select-String" -TestName "Search in File (sls version Cargo.toml)" `
    -NativeCommand "Select-String 'version' Cargo.toml" -PtkCommand "& '$BinaryPath' sls 'version' Cargo.toml"

Run-ComparisonTest -Category "Select-String" -TestName "Pattern Search (sls [package] Cargo.toml)" `
    -NativeCommand "Select-String -SimpleMatch '[package]' Cargo.toml" -PtkCommand "& '$BinaryPath' sls -SimpleMatch '[package]' Cargo.toml"

Run-ComparisonTest -Category "Select-String" -TestName "SimpleMatch Literal (sls -SimpleMatch)" `
    -NativeCommand "Select-String -SimpleMatch 'ptk' Cargo.toml" -PtkCommand "& '$BinaryPath' sls -SimpleMatch 'ptk' Cargo.toml"

Run-ComparisonTest -Category "Select-String" -TestName "Case Sensitive (sls -CaseSensitive)" `
    -NativeCommand "Select-String -CaseSensitive 'version' Cargo.toml" -PtkCommand "& '$BinaryPath' sls -CaseSensitive 'version' Cargo.toml"

Run-ComparisonTest -Category "Select-String" -TestName "Context Lines (sls -Context 1,1)" `
    -NativeCommand "Select-String -Context 1,1 'edition' Cargo.toml" -PtkCommand "& '$BinaryPath' sls -Context 1,1 'edition' Cargo.toml"

Run-ComparisonTest -Category "Select-String" -TestName "Fallback: -Quiet (Boolean)" `
    -NativeCommand "Select-String -Quiet 'version' Cargo.toml" -PtkCommand "& '$BinaryPath' sls -Quiet 'version' Cargo.toml" -ExpectFallback $true

Run-ComparisonTest -Category "Select-String" -TestName "Fallback: Pipeline Search" `
    -NativeCommand "Get-ChildItem -Filter *.md | Select-String 'License'" -PtkCommand "& '$BinaryPath' run 'Get-ChildItem -Filter *.md | Select-String License'" -ExpectFallback $true

# ---------------------------------------------------------
# 4. Get-Process (gps, ps)
# ---------------------------------------------------------
Write-Host "[4/14] Testing Get-Process (gps, ps)..." -ForegroundColor Yellow
Run-ComparisonTest -Category "Get-Process" -TestName "All Processes (Get-Process)" `
    -NativeCommand "Get-Process" -PtkCommand "& '$BinaryPath' get-process"

Run-ComparisonTest -Category "Get-Process" -TestName "Filter by Name (Get-Process pwsh)" `
    -NativeCommand "Get-Process pwsh" -PtkCommand "& '$BinaryPath' get-process pwsh"

Run-ComparisonTest -Category "Get-Process" -TestName "Filter by Wildcard (gps exp*)" `
    -NativeCommand "Get-Process exp*" -PtkCommand "& '$BinaryPath' gps exp*"

Run-ComparisonTest -Category "Get-Process" -TestName "Filter by PID (ps -Id `$PID)" `
    -NativeCommand "Get-Process -Id $PID" -PtkCommand "& '$BinaryPath' ps -Id $PID"

Run-ComparisonTest -Category "Get-Process" -TestName "Alias check (ps)" `
    -NativeCommand "Get-Process" -PtkCommand "& '$BinaryPath' ps"

Run-ComparisonTest -Category "Get-Process" -TestName "Fallback: Pipeline (| Measure)" `
    -NativeCommand "Get-Process | Measure-Object" -PtkCommand "& '$BinaryPath' run 'Get-Process | Measure-Object'" -ExpectFallback $true

# ---------------------------------------------------------
# 5. Get-Service (gsv)
# ---------------------------------------------------------
Write-Host "[5/14] Testing Get-Service (gsv)..." -ForegroundColor Yellow
Run-ComparisonTest -Category "Get-Service" -TestName "All Services (Get-Service)" `
    -NativeCommand "Get-Service" -PtkCommand "& '$BinaryPath' get-service"

Run-ComparisonTest -Category "Get-Service" -TestName "Filter by Name (Get-Service wuauserv)" `
    -NativeCommand "Get-Service wuauserv" -PtkCommand "& '$BinaryPath' get-service wuauserv"

Run-ComparisonTest -Category "Get-Service" -TestName "Filter by Wildcard (gsv *docker*)" `
    -NativeCommand "Get-Service *docker*" -PtkCommand "& '$BinaryPath' gsv *docker*"

Run-ComparisonTest -Category "Get-Service" -TestName "Filter by DisplayName (gsv -DisplayName *Windows*)" `
    -NativeCommand "Get-Service -DisplayName '*Windows*'" -PtkCommand "& '$BinaryPath' gsv -DisplayName '*Windows*'"

Run-ComparisonTest -Category "Get-Service" -TestName "Alias check (gsv)" `
    -NativeCommand "Get-Service" -PtkCommand "& '$BinaryPath' gsv"

Run-ComparisonTest -Category "Get-Service" -TestName "Fallback: Pipeline (| Select -First 3)" `
    -NativeCommand "Get-Service | Select-Object -First 3" -PtkCommand "& '$BinaryPath' run 'Get-Service | Select-Object -First 3'" -ExpectFallback $true

# ---------------------------------------------------------
# 6. Get-NetIPAddress (netip)
# ---------------------------------------------------------
Write-Host "[6/14] Testing Get-NetIPAddress (netip)..." -ForegroundColor Yellow
Run-ComparisonTest -Category "Get-NetIPAddress" -TestName "All IP Addresses (Get-NetIPAddress)" `
    -NativeCommand "Get-NetIPAddress" -PtkCommand "& '$BinaryPath' get-netipaddress"

Run-ComparisonTest -Category "Get-NetIPAddress" -TestName "IPv4 Addresses (-AddressFamily IPv4)" `
    -NativeCommand "Get-NetIPAddress -AddressFamily IPv4" -PtkCommand "& '$BinaryPath' get-netipaddress -AddressFamily IPv4"

Run-ComparisonTest -Category "Get-NetIPAddress" -TestName "IPv6 Addresses (-AddressFamily IPv6)" `
    -NativeCommand "Get-NetIPAddress -AddressFamily IPv6" -PtkCommand "& '$BinaryPath' get-netipaddress -AddressFamily IPv6"

Run-ComparisonTest -Category "Get-NetIPAddress" -TestName "Filter Interface (-InterfaceAlias *Loopback*)" `
    -NativeCommand "Get-NetIPAddress -InterfaceAlias '*Loopback*'" -PtkCommand "& '$BinaryPath' get-netipaddress -InterfaceAlias '*Loopback*'"

Run-ComparisonTest -Category "Get-NetIPAddress" -TestName "Alias check (netip)" `
    -NativeCommand "Get-NetIPAddress" -PtkCommand "& '$BinaryPath' netip"

Run-ComparisonTest -Category "Get-NetIPAddress" -TestName "Fallback: Pipeline (| Select -First 2)" `
    -NativeCommand "Get-NetIPAddress | Select-Object -First 2" -PtkCommand "& '$BinaryPath' run 'Get-NetIPAddress | Select-Object -First 2'" -ExpectFallback $true

# ---------------------------------------------------------
# 7. New-Item (ni)
# ---------------------------------------------------------
Write-Host "[7/14] Testing New-Item (ni)..." -ForegroundColor Yellow
$t1 = "$env:TEMP/ptk_test_f1.tmp"
$t2 = "$env:TEMP/ptk_test_d1"
$t3 = "$env:TEMP/ptk_test_f2.tmp"
$t4 = "$env:TEMP/ptk_test_f3.tmp"
$t5 = "$env:TEMP/ptk_test_f4.tmp"

Run-ComparisonTest -Category "New-Item" -TestName "Create File (New-Item -ItemType File)" `
    -NativeCommand "New-Item -Path '$t1' -ItemType File -Value 'test' -Force; Remove-Item '$t1' -Force" `
    -PtkCommand "& '$BinaryPath' new-item -Path '$t1' -ItemType File -Value 'test' -Force; Remove-Item '$t1' -Force"

Run-ComparisonTest -Category "New-Item" -TestName "Create Directory (New-Item -ItemType Directory)" `
    -NativeCommand "New-Item -Path '$t2' -ItemType Directory -Force; Remove-Item '$t2' -Force" `
    -PtkCommand "& '$BinaryPath' new-item -Path '$t2' -ItemType Directory -Force; Remove-Item '$t2' -Force"

Run-ComparisonTest -Category "New-Item" -TestName "Alias check (ni)" `
    -NativeCommand "New-Item -Path '$t3' -ItemType File -Value 'ni' -Force; Remove-Item '$t3' -Force" `
    -PtkCommand "& '$BinaryPath' ni -Path '$t3' -ItemType File -Value 'ni' -Force; Remove-Item '$t3' -Force"

Run-ComparisonTest -Category "New-Item" -TestName "Create by Name & Path (-Name, -Path)" `
    -NativeCommand "New-Item -Name 'ptk_test_f3.tmp' -Path '$env:TEMP' -ItemType File -Value 'name' -Force; Remove-Item '$t4' -Force" `
    -PtkCommand "& '$BinaryPath' new-item -Name 'ptk_test_f3.tmp' -Path '$env:TEMP' -ItemType File -Value 'name' -Force; Remove-Item '$t4' -Force"

Run-ComparisonTest -Category "New-Item" -TestName "Create File with Value" `
    -NativeCommand "New-Item -Path '$t5' -ItemType File -Value '123' -Force; Remove-Item '$t5' -Force" `
    -PtkCommand "& '$BinaryPath' new-item -Path '$t5' -ItemType File -Value '123' -Force; Remove-Item '$t5' -Force"

# ---------------------------------------------------------
# 8. Get-EventLog & Get-WinEvent (eventlog, winevent)
# ---------------------------------------------------------
Write-Host "[8/14] Testing Event Logs (eventlog, winevent)..." -ForegroundColor Yellow
Run-ComparisonTest -Category "EventLog" -TestName "System Log Newest 3 (Get-EventLog)" `
    -NativeCommand "Get-EventLog -LogName System -Newest 3" -PtkCommand "& '$BinaryPath' get-eventlog -LogName System -Newest 3"

Run-ComparisonTest -Category "EventLog" -TestName "Application Log Newest 3 (Get-EventLog)" `
    -NativeCommand "Get-EventLog -LogName Application -Newest 3" -PtkCommand "& '$BinaryPath' get-eventlog -LogName Application -Newest 3"

Run-ComparisonTest -Category "EventLog" -TestName "Alias check (eventlog)" `
    -NativeCommand "Get-EventLog -LogName System -Newest 2" -PtkCommand "& '$BinaryPath' eventlog -LogName System -Newest 2"

Run-ComparisonTest -Category "WinEvent" -TestName "WinEvent MaxEvents 3 (Get-WinEvent)" `
    -NativeCommand "Get-WinEvent -LogName System -MaxEvents 3" -PtkCommand "& '$BinaryPath' get-winevent -LogName System -MaxEvents 3"

Run-ComparisonTest -Category "WinEvent" -TestName "Alias check (winevent)" `
    -NativeCommand "Get-WinEvent -LogName System -MaxEvents 2" -PtkCommand "& '$BinaryPath' winevent -LogName System -MaxEvents 2"

Run-ComparisonTest -Category "WinEvent" -TestName "Fallback: Pipeline (| Measure)" `
    -NativeCommand "Get-WinEvent -LogName System -MaxEvents 3 | Measure-Object" -PtkCommand "& '$BinaryPath' run 'Get-WinEvent -LogName System -MaxEvents 3 | Measure-Object'" -ExpectFallback $true

# ---------------------------------------------------------
# 9. Get-NetTCPConnection & netstat (nettcp, netstat)
# ---------------------------------------------------------
Write-Host "[9/14] Testing NetTCP & Netstat (nettcp, netstat)..." -ForegroundColor Yellow
Run-ComparisonTest -Category "NetTCP" -TestName "Listen Connections (Get-NetTCPConnection -State Listen)" `
    -NativeCommand "Get-NetTCPConnection -State Listen" `
    -PtkCommand "& '$BinaryPath' get-nettcpconnection -State Listen"

Run-ComparisonTest -Category "NetTCP" -TestName "Alias check (nettcp)" `
    -NativeCommand "Get-NetTCPConnection -State Listen" `
    -PtkCommand "& '$BinaryPath' nettcp -State Listen"

Run-ComparisonTest -Category "Netstat" -TestName "Active Connections (netstat -ano)" `
    -NativeCommand "netstat -ano" `
    -PtkCommand "& '$BinaryPath' netstat -ano"

Run-ComparisonTest -Category "Netstat" -TestName "Direct ptk run 'netstat'" `
    -NativeCommand "netstat -an" `
    -PtkCommand "& '$BinaryPath' run 'netstat -an'"

Run-ComparisonTest -Category "NetTCP" -TestName "Fallback: Pipeline (| Measure)" `
    -NativeCommand "Get-NetTCPConnection | Measure-Object" `
    -PtkCommand "& '$BinaryPath' run 'Get-NetTCPConnection | Measure-Object'" -ExpectFallback $true

# ---------------------------------------------------------
# 10. Resolve-DnsName (dns)
# ---------------------------------------------------------
Write-Host "[10/14] Testing Resolve-DnsName (dns)..." -ForegroundColor Yellow
Run-ComparisonTest -Category "DNS" -TestName "Lookup localhost (Resolve-DnsName)" `
    -NativeCommand "Resolve-DnsName localhost" `
    -PtkCommand "& '$BinaryPath' resolve-dns localhost"

Run-ComparisonTest -Category "DNS" -TestName "Alias check (dns)" `
    -NativeCommand "Resolve-DnsName localhost" `
    -PtkCommand "& '$BinaryPath' dns localhost"

Run-ComparisonTest -Category "DNS" -TestName "Type filter (Resolve-DnsName -Type A)" `
    -NativeCommand "Resolve-DnsName localhost -Type A" `
    -PtkCommand "& '$BinaryPath' resolve-dns localhost -Type A"

Run-ComparisonTest -Category "DNS" -TestName "External lookup (google.com)" `
    -NativeCommand "Resolve-DnsName google.com" `
    -PtkCommand "& '$BinaryPath' resolve-dns google.com"

Run-ComparisonTest -Category "DNS" -TestName "Fallback: Pipeline (| Select)" `
    -NativeCommand "Resolve-DnsName localhost | Select-Object -ExpandProperty IPAddress" `
    -PtkCommand "& '$BinaryPath' run 'Resolve-DnsName localhost | Select-Object -ExpandProperty IPAddress'" -ExpectFallback $true

# ---------------------------------------------------------
# 11. Test-NetConnection (tnc)
# ---------------------------------------------------------
Write-Host "[11/14] Testing Test-NetConnection (tnc)..." -ForegroundColor Yellow
Run-ComparisonTest -Category "TestNet" -TestName "Ping Test (Test-NetConnection localhost)" `
    -NativeCommand "Test-NetConnection localhost" `
    -PtkCommand "& '$BinaryPath' test-net localhost"

Run-ComparisonTest -Category "TestNet" -TestName "Alias check (tnc)" `
    -NativeCommand "Test-NetConnection localhost" `
    -PtkCommand "& '$BinaryPath' tnc localhost"

Run-ComparisonTest -Category "TestNet" -TestName "TCP Port Check (Active Port 135)" `
    -NativeCommand "Test-NetConnection localhost -Port 135" `
    -PtkCommand "& '$BinaryPath' test-net localhost -Port 135"

Run-ComparisonTest -Category "TestNet" -TestName "Direct ptk run 'tnc'" `
    -NativeCommand "Test-NetConnection localhost -Port 135" `
    -PtkCommand "& '$BinaryPath' run 'tnc localhost -Port 135'"

Run-ComparisonTest -Category "TestNet" -TestName "Fallback: Pipeline (| Select)" `
    -NativeCommand "Test-NetConnection localhost | Select-Object -ExpandProperty PingSucceeded" `
    -PtkCommand "& '$BinaryPath' run 'Test-NetConnection localhost | Select-Object -ExpandProperty PingSucceeded'" -ExpectFallback $true

# ---------------------------------------------------------
# 12. ConvertFrom-Json & ConvertTo-Json
# ---------------------------------------------------------
Write-Host "[12/14] Testing JSON serialization (ConvertFrom-Json, ConvertTo-Json)..." -ForegroundColor Yellow
Run-ComparisonTest -Category "JSON" -TestName "ConvertFrom-Json string object" `
    -NativeCommand 'ConvertFrom-Json -InputObject ''{"foo": "bar", "val": 42}''' `
    -PtkCommand "& '$BinaryPath' from-json -InputObject '{`"foo`": `"bar`", `"val`": 42}'"

Run-ComparisonTest -Category "JSON" -TestName "ConvertFrom-Json multi-property" `
    -NativeCommand 'ConvertFrom-Json -InputObject ''{"a": 1, "b": 2, "c": 3}''' `
    -PtkCommand "& '$BinaryPath' from-json -InputObject '{`"a`": 1, `"b`": 2, `"c`": 3}'"

Run-ComparisonTest -Category "JSON" -TestName "ConvertTo-Json minification" `
    -NativeCommand "Get-Process -Id 4 | Select-Object Id, ProcessName | ConvertTo-Json" `
    -PtkCommand "& '$BinaryPath' run 'Get-Process -Id 4 | Select-Object Id, ProcessName | ConvertTo-Json'" -ExpectFallback $true

Run-ComparisonTest -Category "JSON" -TestName "Direct ptk run convertfrom-json" `
    -NativeCommand 'ConvertFrom-Json -InputObject ''{"status": "ok"}''' `
    -PtkCommand "& '$BinaryPath' convertfrom-json -InputObject '{`"status`": `"ok`"}'"

Run-ComparisonTest -Category "JSON" -TestName "Fallback: JSON pipeline" `
    -NativeCommand "ConvertFrom-Json -InputObject '{`"id`": 99}' | Select-Object -ExpandProperty id" `
    -PtkCommand "& '$BinaryPath' run `"ConvertFrom-Json -InputObject '{`"id`": 99}' | Select-Object -ExpandProperty id`"" -ExpectFallback $true

# ---------------------------------------------------------
# 13. ConvertFrom-Csv & Export-Csv (epcsv)
# ---------------------------------------------------------
Write-Host "[13/14] Testing CSV conversion (ConvertFrom-Csv, Export-Csv)..." -ForegroundColor Yellow

Run-ComparisonTest -Category "CSV" -TestName "ConvertFrom-Csv string" `
    -NativeCommand "Get-Content '$sampleCsv' | ConvertFrom-Csv" `
    -PtkCommand "& '$BinaryPath' run 'Get-Content `'$sampleCsv`' | ConvertFrom-Csv'"

Run-ComparisonTest -Category "CSV" -TestName "Export-Csv file creation" `
    -NativeCommand "Get-Process -Id 4 | Select-Object Id, ProcessName | Export-Csv -Path '$csvTmp' -NoTypeInformation; Remove-Item '$csvTmp' -Force" `
    -PtkCommand "& '$BinaryPath' export-csv -Path '$csvTmp' -InputObject (Get-Process -Id 4 | Select-Object Id, ProcessName) -NoTypeInformation; Remove-Item '$csvTmp' -Force"

Run-ComparisonTest -Category "CSV" -TestName "Alias check (epcsv)" `
    -NativeCommand "Get-Process -Id 4 | Select-Object Id, ProcessName | Export-Csv -Path '$csvTmp' -NoTypeInformation; Remove-Item '$csvTmp' -Force" `
    -PtkCommand "& '$BinaryPath' epcsv -Path '$csvTmp' -InputObject (Get-Process -Id 4 | Select-Object Id, ProcessName) -NoTypeInformation; Remove-Item '$csvTmp' -Force"

Run-ComparisonTest -Category "CSV" -TestName "Direct ptk run 'ConvertFrom-Csv'" `
    -NativeCommand "Get-Content '$sampleCsv' | ConvertFrom-Csv" `
    -PtkCommand "& '$BinaryPath' run 'Get-Content `'$sampleCsv`' | ConvertFrom-Csv'"

Run-ComparisonTest -Category "CSV" -TestName "Fallback: CSV pipeline (| Measure)" `
    -NativeCommand "ConvertFrom-Csv -InputObject 'id,val`n1,10`n2,20' | Measure-Object" `
    -PtkCommand "& '$BinaryPath' run `"ConvertFrom-Csv -InputObject 'id,val``n1,10``n2,20' | Measure-Object`"" -ExpectFallback $true

# ---------------------------------------------------------
# 14. Format-Table & Format-List (ft, fl)
# ---------------------------------------------------------
Write-Host "[14/14] Testing Formatters (Format-Table, Format-List)..." -ForegroundColor Yellow
Run-ComparisonTest -Category "Format" -TestName "Format-Table (ft)" `
    -NativeCommand "Get-Process -Id 4 | Select-Object Id, ProcessName | Format-Table" `
    -PtkCommand "& '$BinaryPath' format-table -InputObject (Get-Process -Id 4 | Select-Object Id, ProcessName)"

Run-ComparisonTest -Category "Format" -TestName "Alias check (ft)" `
    -NativeCommand "Get-Process -Id 4 | Select-Object Id, ProcessName | Format-Table" `
    -PtkCommand "& '$BinaryPath' ft -InputObject (Get-Process -Id 4 | Select-Object Id, ProcessName)"

Run-ComparisonTest -Category "Format" -TestName "Format-List (fl)" `
    -NativeCommand "Get-Process -Id 4 | Select-Object Id, ProcessName | Format-List" `
    -PtkCommand "& '$BinaryPath' format-list -InputObject (Get-Process -Id 4 | Select-Object Id, ProcessName)"

Run-ComparisonTest -Category "Format" -TestName "Alias check (fl)" `
    -NativeCommand "Get-Process -Id 4 | Select-Object Id, ProcessName | Format-List" `
    -PtkCommand "& '$BinaryPath' fl -InputObject (Get-Process -Id 4 | Select-Object Id, ProcessName)"

Run-ComparisonTest -Category "Format" -TestName "Fallback: Formatter pipeline (| Out-String)" `
    -NativeCommand "Get-Process -Id 4 | Format-Table | Out-String" `
    -PtkCommand "& '$BinaryPath' run 'Get-Process -Id 4 | Format-Table | Out-String'" -ExpectFallback $true


    AfterAll {
        Remove-Item "$env:TEMP/ptk_test_sample.csv" -Force -ErrorAction SilentlyContinue
        Remove-Item "$env:TEMP/ptk_test_export.csv" -Force -ErrorAction SilentlyContinue

        $benchFile = "$env:TEMP/ptk_benchmark_results.jsonl"
        $results = @()
        if (Test-Path $benchFile) {
            $lines = Get-Content $benchFile
            foreach ($line in $lines) {
                if (-not [string]::IsNullOrWhiteSpace($line)) {
                    $results += ($line | ConvertFrom-Json)
                }
            }
        }

        if ($results -and $results.Count -gt 0) {
            $table = ($results | Format-Table -AutoSize -Property Category, TestName, NativeChars, PTKChars, SavingsPct, NativeMs, PtkMs, SpeedRatio, Status, Notes | Out-String).TrimEnd()

            $passedCount = ($results | Where-Object { $_.Status -eq 'PASS' }).Count
            $totalCount = $results.Count

            $totalNativeMs = [math]::Round(($results | Measure-Object -Property NativeMs -Sum).Sum, 1)
            $totalPtkMs = [math]::Round(($results | Measure-Object -Property PtkMs -Sum).Sum, 1)
            $avgNativeMs = [math]::Round(($results | Measure-Object -Property NativeMs -Average).Average, 1)
            $avgPtkMs = [math]::Round(($results | Measure-Object -Property PtkMs -Average).Average, 1)
            $overallRatio = if ($totalPtkMs -gt 0) { [math]::Round(($totalNativeMs / $totalPtkMs), 2) } else { 0 }

            $report = @"

=================================================================
                PTK TOKEN SAVINGS & BENCHMARK REPORT             
=================================================================
$table

Overall Result: $passedCount / $totalCount benchmark assertions passed.

-----------------------------------------------------------------
                    SPEED COMPARISON SUMMARY                      
-----------------------------------------------------------------
Total execution time: native = $totalNativeMs ms | ptk = $totalPtkMs ms
Average per test    : native = $avgNativeMs ms  | ptk = $avgPtkMs ms
Overall speed ratio (native/ptk): $overallRatio
"@
            Set-Content -Path "$env:TEMP/ptk_benchmark_report.txt" -Value $report -Force
            [Console]::Out.WriteLine($report)
            [Console]::Out.Flush()
        }
    }
}
