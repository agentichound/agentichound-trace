$ErrorActionPreference = "Stop"

$examplesDir = Join-Path $PSScriptRoot "..\\schemas\\examples"
$examplesDir = (Resolve-Path $examplesDir).Path

Get-ChildItem -Path $examplesDir -File -Filter *.json | Remove-Item -Force

function Format-Ts {
    param([datetime]$Dt)
    return $Dt.ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
}

function Write-TraceFile {
    param(
        [string]$FileName,
        [string]$Scenario,
        [int]$Index,
        [bool]$IsFailure,
        [bool]$HasRetry,
        [bool]$ToolBottleneck
    )

    $runId = "run_${Scenario}_${Index}"
    $rootSpan = "spn_${Scenario}_${Index}_root"
    $modelSpan = "spn_${Scenario}_${Index}_model"
    $toolSpan = "spn_${Scenario}_${Index}_tool"
    $retrySpan = "spn_${Scenario}_${Index}_retry"

    $base = [datetime]::Parse("2026-04-19T18:00:00Z").AddMinutes($Index)
    $runDuration = if ($ToolBottleneck) { 2400 } elseif ($HasRetry) { 2100 } elseif ($IsFailure) { 1200 } else { 1400 }

    $runStatus = if ($IsFailure) { "error" } else { "ok" }
    $rootStatus = if ($IsFailure) { "error" } else { "ok" }

    $toolEndOffset = if ($ToolBottleneck) { 2200 } elseif ($IsFailure) { 1000 } else { 1100 }
    $toolDuration = if ($ToolBottleneck) { 1700 } elseif ($IsFailure) { 500 } else { 600 }

    $spans = @(
        [ordered]@{
            span_id = $rootSpan
            run_id = $runId
            parent_span_id = $null
            retry_of_span_id = $null
            kind = "orchestration"
            name = "agent.execute"
            started_at = Format-Ts $base
            ended_at = Format-Ts ($base.AddMilliseconds($runDuration))
            duration_ms = $runDuration
            status = $rootStatus
        },
        [ordered]@{
            span_id = $modelSpan
            run_id = $runId
            parent_span_id = $rootSpan
            retry_of_span_id = $null
            kind = "model"
            name = "model.plan"
            started_at = Format-Ts ($base.AddMilliseconds(100))
            ended_at = Format-Ts ($base.AddMilliseconds(450))
            duration_ms = 350
            status = "ok"
        },
        [ordered]@{
            span_id = $toolSpan
            run_id = $runId
            parent_span_id = $rootSpan
            retry_of_span_id = $null
            kind = "tool"
            name = "tool.fetch"
            started_at = Format-Ts ($base.AddMilliseconds(500))
            ended_at = Format-Ts ($base.AddMilliseconds($toolEndOffset))
            duration_ms = $toolDuration
            status = $(if ($IsFailure) { "error" } else { "ok" })
        }
    )

    if ($HasRetry) {
        $spans += [ordered]@{
            span_id = $retrySpan
            run_id = $runId
            parent_span_id = $rootSpan
            retry_of_span_id = $toolSpan
            kind = "retry"
            name = "retry.tool.fetch"
            started_at = Format-Ts ($base.AddMilliseconds(1200))
            ended_at = Format-Ts ($base.AddMilliseconds(1800))
            duration_ms = 600
            status = "ok"
        }
    }

    $events = @(
        [ordered]@{
            event_id = "evt_${Scenario}_${Index}_start"
            run_id = $runId
            span_id = $rootSpan
            kind = "orchestration"
            ts = Format-Ts $base
            payload = [ordered]@{ phase = "start" }
        },
        [ordered]@{
            event_id = "evt_${Scenario}_${Index}_model"
            run_id = $runId
            span_id = $modelSpan
            kind = "model"
            ts = Format-Ts ($base.AddMilliseconds(200))
            payload = [ordered]@{ provider = "openai"; model = "gpt-4.1-mini" }
        },
        [ordered]@{
            event_id = "evt_${Scenario}_${Index}_tool"
            run_id = $runId
            span_id = $toolSpan
            kind = "tool"
            ts = Format-Ts ($base.AddMilliseconds(700))
            payload = [ordered]@{ tool = "fetch"; endpoint = "inventory" }
        }
    )

    if ($HasRetry) {
        $events += [ordered]@{
            event_id = "evt_${Scenario}_${Index}_retry"
            run_id = $runId
            span_id = $retrySpan
            kind = "retry"
            ts = Format-Ts ($base.AddMilliseconds(1300))
            payload = [ordered]@{ reason = "timeout" }
        }
    }

    $errors = @()
    if ($IsFailure) {
        $errors += [ordered]@{
            error_id = "err_${Scenario}_${Index}_tool"
            run_id = $runId
            span_id = $toolSpan
            ts = Format-Ts ($base.AddMilliseconds(900))
            code = "TOOL_TIMEOUT"
            message = "Tool call timed out"
            retryable = $true
        }
    }

    $usage = @(
        [ordered]@{
            usage_id = "use_${Scenario}_${Index}_model"
            run_id = $runId
            span_id = $modelSpan
            kind = "model"
            prompt_tokens = 120
            completion_tokens = 80
            total_tokens = 200
            estimated_cost_usd = 0.0024
            currency = "USD"
        },
        [ordered]@{
            usage_id = "use_${Scenario}_${Index}_tool"
            run_id = $runId
            span_id = $toolSpan
            kind = "tool"
            prompt_tokens = 0
            completion_tokens = 0
            total_tokens = 0
            estimated_cost_usd = $(if ($ToolBottleneck) { 0.0011 } else { 0.0004 })
            currency = "USD"
        }
    )

    $trace = [ordered]@{
        schema_version = "v0"
        run = [ordered]@{
            run_id = $runId
            started_at = Format-Ts $base
            ended_at = Format-Ts ($base.AddMilliseconds($runDuration))
            duration_ms = $runDuration
            status = $runStatus
        }
        spans = $spans
        events = $events
        errors = $errors
        usage = $usage
    }

    $trace | ConvertTo-Json -Depth 8 | Set-Content -Encoding ascii -Path (Join-Path $examplesDir $FileName)
}

1..9 | ForEach-Object { Write-TraceFile -FileName ("happy-path-{0:d2}.json" -f $_) -Scenario "happy" -Index $_ -IsFailure $false -HasRetry $false -ToolBottleneck $false }
1..9 | ForEach-Object { Write-TraceFile -FileName ("tool-bottleneck-{0:d2}.json" -f $_) -Scenario "tb" -Index $_ -IsFailure $false -HasRetry $false -ToolBottleneck $true }
1..9 | ForEach-Object { Write-TraceFile -FileName ("retry-then-success-{0:d2}.json" -f $_) -Scenario "retry" -Index $_ -IsFailure $false -HasRetry $true -ToolBottleneck $false }
1..8 | ForEach-Object { Write-TraceFile -FileName ("failed-run-{0:d2}.json" -f $_) -Scenario "fail" -Index $_ -IsFailure $true -HasRetry $false -ToolBottleneck $false }

$invalid = [ordered]@{
    schema_version = "v0"
    run = [ordered]@{
        started_at = "2026-04-19T18:00:00Z"
        ended_at = "2026-04-19T18:00:01Z"
        duration_ms = 1000
        status = "ok"
    }
    spans = @()
    events = @()
    errors = @()
    usage = @()
}

$invalid | ConvertTo-Json -Depth 8 | Set-Content -Encoding ascii -Path (Join-Path $examplesDir "invalid-missing-run-id.json")
