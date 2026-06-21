using System;
using System.Diagnostics;
using System.Text;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Threading;

namespace manager;

public partial class ProcessWindow : Window
{
    private readonly Process _process = new();
    private readonly StringBuilder _output = new();

    public ProcessWindow()
    {
        InitializeComponent();
    }

    public ProcessWindow(string fileName, string? arguments = null, string? workingDir = null)
        : this()
    {
        Title = $"{fileName} - Process Output";
        CloseButton.Click += (_, _) => Close();

        _process.StartInfo = new ProcessStartInfo
        {
            FileName = fileName,
            Arguments = arguments ?? "",
            WorkingDirectory = workingDir ?? "",
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true,
            StandardOutputEncoding = Encoding.UTF8,
            StandardErrorEncoding = Encoding.UTF8,
        };

        _process.EnableRaisingEvents = true;
        _process.Exited += (_, _) =>
        {
            _process.WaitForExit();
            Dispatcher.UIThread.Post(() =>
            {
                StatusText.Text = $"Exited (Code: {_process.ExitCode})";
                AppendOutput($"[Process exited, code: {_process.ExitCode}]");
            });
        };

        StartProcess();
    }

    private async void StartProcess()
    {
        try
        {
            StatusText.Text = "Starting...";
            _process.Start();

            StatusText.Text = $"Running (PID: {_process.Id})";

            var stdoutTask = ReadStreamAsync(_process.StandardOutput);
            var stderrTask = ReadStreamAsync(_process.StandardError);
            await Task.WhenAll(stdoutTask, stderrTask);
        }
        catch (Exception ex)
        {
            Dispatcher.UIThread.Post(() =>
            {
                AppendOutput($"[Start failed] {ex.Message}");
                StatusText.Text = "Start failed";
            });
        }
    }

    private async Task ReadStreamAsync(System.IO.StreamReader reader)
    {
        var buffer = new char[4096];
        int charsRead;
        while ((charsRead = await reader.ReadAsync(buffer, 0, buffer.Length)) > 0)
        {
            var segment = new string(buffer, 0, charsRead);
            Dispatcher.UIThread.Post(() => AppendOutput(segment));
        }
    }

    private void AppendOutput(string text)
    {
        _output.Append(text);
        OutputText.Text = _output.ToString();

        OutputScroll.ScrollToEnd();
    }

    protected override void OnClosed(EventArgs e)
    {
        base.OnClosed(e);
        if (!_process.HasExited)
        {
            try { _process.Kill(); } catch { }
        }
        _process.Dispose();
    }
}
