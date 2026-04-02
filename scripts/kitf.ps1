function kitf {
    <#
    .SYNOPSIS
    Friction Log Wizard v1.2.3 (Armor-Plated Edition)
    Dot-source this file in your $PROFILE to enable.
    #>
    try {
        # Unicode Resilience: Force UTF-8 for Vietnamese input
        [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
        [Console]::InputEncoding = [System.Text.Encoding]::UTF8

        Write-Host "`n[🤖 CHẾ ĐỘ SĂN MA SÁT v1.2.3 (BỌC THÉP)]" -ForegroundColor Cyan

        $symptom = Read-Host "1. Triệu chứng (Symptom)"
        $trigger = Read-Host "2. Tác nhân (Trigger)"
        
        $category = Read-Host "3. Phân loại [ux/perf/logic/invariant_clash] (Mặc định: ux)"
        if ([string]::IsNullOrWhiteSpace($category)) { $category = "ux" }
        
        $severity = Read-Host "4. Mức độ [low/med/high/critical] (Mặc định: med)"
        if ([string]::IsNullOrWhiteSpace($severity)) { $severity = "med" }
        
        $expectation = Read-Host "5. Kỳ vọng (Expectation)"

        # Atomic JSON packaging
        $payloadObj = @{
            symptom = $symptom
            trigger = $trigger
            category = $category
            severity = $severity
            expectation = $expectation
        }
        
        $rawJson = $payloadObj | ConvertTo-Json -Depth 3 -Compress

        # EXECUTION: Use the installed wrapper to avoid repo-path drift and stale editable installs
        # Pass the JSON string directly so PowerShell preserves it as one native argument.
        kit learn --tag note --kind friction --content $rawJson

        Write-Host "✅ Đã nạp Friction Log JSON nguyên vẹn vào Két sắt!" -ForegroundColor Green

    } catch {
        Write-Warning "`n⚠️ Thoát chế độ săn ma sát (Ctrl+C) hoặc có lỗi xảy ra."
    }
}
