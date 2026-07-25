# Windows

## The backend is DX12, not Vulkan

`Window::instance` asks for `Backends::DX12` on Windows. Android pins its backend too,
see [android.md](android.md), every other platform keeps the wgpu default. This is not
a preference, Vulkan crashes the process on Intel integrated GPUs.

With every backend enabled wgpu picks Vulkan on Windows. Intel's Vulkan driver
`igvk64.dll` for Gen9 integrated parts, a UHD Graphics 620 for example, faults inside
`vkCreateDevice`. The process dies with `STATUS_ACCESS_VIOLATION`, exit code `0xc0000005`,
with no Rust panic, no backtrace and no log line. The last thing printed is
`Backend: vulkan` from `start_internal`, because the very next call is `request_device`.

Headless dies the same way, so a crash right after that line is never about the window or
the surface.

The Windows event log is what names the culprit, nothing in the process survives to say
it:

```powershell
Get-WinEvent -FilterHashtable @{LogName='Application'; ProviderName='Application Error'} -MaxEvents 5 |
    ForEach-Object { $_.Message }
```

It prints the faulting module, `igvk64.dll` in this case. Use it for any silent
`0xc0000005`, not only this one.

DX12 is not a downgrade here. The same class of GPU reports feature level 12_1, the top
tier it reaches, readable from `HKLM:\SOFTWARE\Microsoft\DirectX` as
`MaxD3D12FeatureLevel`.

## WGPU_BACKEND

`Window::instance` ends with `descriptor.with_env()`, so `WGPU_BACKEND` selects the
backend on any platform. `WGPU_BACKEND=vulkan` brings the old behavior back on a machine
with a working driver.

This did nothing before. The engine built its instance with `Instance::default()`, which
hardcodes `Backends::all()` and never reads the environment, so a backend could not be
chosen by hand at all.
