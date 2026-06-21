using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Media;
using Avalonia.Threading;
using Tomlyn;
using Tomlyn.Model;

namespace manager;

public partial class MainWindow : Window
{
    private readonly string _configDir;
    private readonly string _sourceToml;
    private readonly string _guiConfigPath;
    private bool _dirty;
    private bool _loading;
    private bool _promptingClose;
    private bool _dmvopRunning;
    private Process? _dmvopProcess;
    private readonly StringBuilder _dmvopOutput = new();
    private const string BaseTitle = "DumbVoiceProtocol";

    public MainWindow()
    {
        InitializeComponent();

        // Parent directory of the program location = build/
        _configDir = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, ".."));
        _sourceToml = Path.Combine(_configDir, "dmvop.toml");
        _guiConfigPath = Path.Combine(_configDir, "dmvop-gui.toml");

        DownloadModelButton.Click += OnDownloadModel;
        KeyDown += OnKeyDown;

        // Hide Unix-specific items on Windows
        if (OperatingSystem.IsWindows())
        {
            IpcCheck.IsVisible = false;
            SocketFilePanel.IsVisible = false;
        }

        // Mark dirty on all input control changes
        SubscribeChanges();

        Closing += OnClosing;

        ActionButton.Click += OnActionButton;
        _ = StartDashboardTimer();

        LoadConfig();
    }

    private void SubscribeChanges()
    {
        // TextBoxes
        DeviceBox.TextChanged += (_, _) => MarkDirty();
        FormatBox.TextChanged += (_, _) => MarkDirty();
        LangBox.TextChanged += (_, _) => MarkDirty();
        PortBox.TextChanged += (_, _) => MarkDirty();
        SubnetMaskBox.TextChanged += (_, _) => MarkDirty();
        SocketFileBox.TextChanged += (_, _) => MarkDirty();
        ModelsDirBox.TextChanged += (_, _) => MarkDirty();
        PostBox.TextChanged += (_, _) => MarkDirty();

        // ComboBox
        ModelBox.SelectionChanged += (_, _) => MarkDirty();

        // CheckBoxes
        StdoutCheck.IsCheckedChanged += (_, _) => MarkDirty();
        TcpCheck.IsCheckedChanged += (_, _) => MarkDirty();
        UdpCheck.IsCheckedChanged += (_, _) => MarkDirty();
        UdpBroadcastCheck.IsCheckedChanged += (_, _) => MarkDirty();
        IpcCheck.IsCheckedChanged += (_, _) => MarkDirty();
        InstantCheck.IsCheckedChanged += (_, _) => MarkDirty();
    }

    private void MarkDirty()
    {
        if (_loading || _dirty) return;
        _dirty = true;
        UpdateTitle();
    }

    private void ClearDirty()
    {
        if (!_dirty) return;
        _dirty = false;
        UpdateTitle();
    }

    private void UpdateTitle()
    {
        Title = _dirty ? $"{BaseTitle} *" : BaseTitle;
    }

    // ---- Dashboard ----

    private async Task StartDashboardTimer()
    {
        while (true)
        {
            await Task.Delay(2000);
            CheckDmvopStatus();
        }
    }

    private bool IsDmvopRunning()
    {
        return Process.GetProcessesByName("dmvop").Any(p => p.Id != Environment.ProcessId);
    }

    private void CheckDmvopStatus()
    {
        var running = IsDmvopRunning();
        if (running == _dmvopRunning) return;
        _dmvopRunning = running;
        Dispatcher.UIThread.Post(() =>
        {
            UpdateDashboard();
            if (!_dmvopRunning)
            {
                _dmvopProcess = null;
                _dmvopOutput.Clear();
                OutputArea.Text = "";
            }
        });
    }

    private void UpdateDashboard()
    {
        if (_dmvopRunning)
        {
            StatusLight.Fill = new SolidColorBrush(Colors.LimeGreen);
            StatusTextRight.Text = "Running";
            StatusTextRight.Foreground = new SolidColorBrush(Colors.LimeGreen);
            ActionButton.Content = "Stop";
        }
        else
        {
            StatusLight.Fill = new SolidColorBrush(Colors.Red);
            StatusTextRight.Text = "Stopped";
            StatusTextRight.Foreground = new SolidColorBrush(Colors.Gray);
            ActionButton.Content = "Start";
        }
    }

    private void OnActionButton(object? sender, Avalonia.Interactivity.RoutedEventArgs e)
    {
        if (_dmvopRunning)
        {
            // Stop
            try { _dmvopProcess?.Kill(); } catch { }
            foreach (var proc in Process.GetProcessesByName("dmvop"))
            {
                if (proc.Id == Environment.ProcessId) continue;
                try { proc.Kill(); } catch { }
            }
            _dmvopProcess = null;
        }
        else
        {
            // Start: launch from dmvop.exe directory, no window
            var dmvopPath = Path.Combine(_configDir, "dmvop.exe");
            if (!File.Exists(dmvopPath))
            {
                StatusLabel.Text = "Cannot find dmvop.exe";
                return;
            }

            _dmvopOutput.Clear();
            OutputArea.Text = "";

            var psi = new ProcessStartInfo
            {
                FileName = dmvopPath,
                Arguments = "--config=\"./dmvop-gui.toml\" --verbose",
                WorkingDirectory = _configDir,
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                StandardOutputEncoding = Encoding.UTF8,
                StandardErrorEncoding = Encoding.UTF8,
            };

            try
            {
                var proc = Process.Start(psi);
                if (proc == null)
                {
                    StatusLabel.Text = "Failed to start";
                    return;
                }
                _dmvopProcess = proc;

                // Read stdout and stderr in parallel
                _ = ReadStreamAsync(proc.StandardOutput);
                _ = ReadStreamAsync(proc.StandardError);

                proc.Exited += (_, _) =>
                {
                    proc.WaitForExit();
                    _dmvopRunning = false;
                    Dispatcher.UIThread.Post(() =>
                    {
                        UpdateDashboard();
                        // Don't clear output immediately so the user can see the last content
                    });
                };
                proc.EnableRaisingEvents = true;
            }
            catch (Exception ex)
            {
                StatusLabel.Text = $"Failed to start: {ex.Message}";
            }
        }
    }

    private async Task ReadStreamAsync(StreamReader reader)
    {
        var buffer = new char[4096];
        int charsRead;
        while ((charsRead = await reader.ReadAsync(buffer, 0, buffer.Length)) > 0)
        {
            var segment = new string(buffer, 0, charsRead);
            lock (_dmvopOutput)
            {
                _dmvopOutput.Append(segment);
                // Limit max length to avoid memory blowup
                if (_dmvopOutput.Length > 100_000)
                    _dmvopOutput.Remove(0, _dmvopOutput.Length - 50_000);
            }
            Dispatcher.UIThread.Post(() =>
            {
                OutputArea.Text = _dmvopOutput.ToString();
                // OutputScroll is the ScrollViewer x:Name? Use parent ScrollViewer
                // Find the parent ScrollViewer of OutputArea directly
                if (OutputArea.Parent is ScrollViewer sv)
                    sv.ScrollToEnd();
            });
        }
    }

    private void OnClosing(object? sender, WindowClosingEventArgs e)
    {
        if (!_dirty || _promptingClose) return;

        e.Cancel = true;
        _promptingClose = true;

        Dispatcher.UIThread.Post(async () =>
        {
            var result = await MessageBox.Show(this,
                "There are unsaved changes. Do you want to save?", "DumbVoiceProtocol",
                ("Save", MessageBoxResult.Yes),
                ("Don't Save", MessageBoxResult.No),
                ("Cancel", MessageBoxResult.Cancel));

            _promptingClose = false;

            switch (result)
            {
                case MessageBoxResult.Yes:
                    OnSave();
                    if (!_dirty)
                        Close();
                    break;
                case MessageBoxResult.No:
                    _dirty = false;
                    Close();
                    break;
            }
        });
    }

    private void LoadConfig()
    {
        _loading = true;

        if (!File.Exists(_guiConfigPath))
        {
            if (File.Exists(_sourceToml))
            {
                File.Copy(_sourceToml, _guiConfigPath);
            }
            else
            {
                MessageBox.Show(this, "dmvop.toml does not exist", "Configuration Error",
                    ("OK", MessageBoxResult.Ok));
                StatusLabel.Text = "dmvop.toml does not exist";
                return;
            }
        }

        try
        {
            var text = File.ReadAllText(_guiConfigPath);
            var model = Toml.ToModel(text);

            DeviceBox.Text = GetString(model, "device") ?? "auto";
            FormatBox.Text = GetString(model, "format") ?? "%{vol},%{word}";

            var modelName = GetString(model, "model") ?? "base.en";
            SelectComboBoxItem(ModelBox, modelName);

            LangBox.Text = GetString(model, "lang") ?? "en";

            if (model.TryGetValue("output", out var outputVal) && outputVal is TomlArray outputArray)
            {
                var modes = outputArray.OfType<string>().ToList();
                StdoutCheck.IsChecked = modes.Contains("stdout");
                TcpCheck.IsChecked = modes.Contains("tcp");
                UdpCheck.IsChecked = modes.Contains("udp");
                UdpBroadcastCheck.IsChecked = modes.Contains("udp-broadcast");
                IpcCheck.IsChecked = modes.Contains("ipc");
            }

            if (model.TryGetValue("port", out var portVal) && portVal is long portLong)
                PortBox.Text = portLong.ToString();

            if (model.TryGetValue("instant", out var instantVal) && instantVal is bool instant)
                InstantCheck.IsChecked = instant;

            SubnetMaskBox.Text = GetString(model, "subnet_mask") ?? "255.255.255.0";
            SocketFileBox.Text = GetString(model, "socket_file") ?? "./dmvop.sock";
            ModelsDirBox.Text = GetString(model, "models_dir") ?? "";
            PostBox.Text = GetString(model, "post") ?? "";

            StatusLabel.Text = $"Loaded: {Path.GetFileName(_guiConfigPath)}";
        }
        catch (Exception ex)
        {
            StatusLabel.Text = $"Load error: {ex.Message}";
        }
        finally
        {
            _loading = false;
            ClearDirty();
        }
    }

    private void OnDownloadModel(object? sender, Avalonia.Interactivity.RoutedEventArgs e)
    {
        var modelName = GetComboBoxText(ModelBox);
        if (string.IsNullOrEmpty(modelName))
        {
            StatusLabel.Text = "Please select a model first";
            return;
        }

        var dmvopPath = Path.Combine(_configDir, "dmvop.exe");
        if (!File.Exists(dmvopPath))
        {
            StatusLabel.Text = "Cannot find dmvop.exe";
            return;
        }

        var procWin = new ProcessWindow(dmvopPath, $"--download-model={modelName}", _configDir);
        procWin.ShowDialog(this);
    }

    private void OnKeyDown(object? sender, Avalonia.Input.KeyEventArgs e)
    {
        if (e.Key == Avalonia.Input.Key.S && e.KeyModifiers.HasFlag(Avalonia.Input.KeyModifiers.Control))
        {
            OnSave();
        }
    }

    private void OnSave()
    {
        try
        {
            var model = new TomlTable();

            model["device"] = DeviceBox.Text ?? "auto";
            model["format"] = FormatBox.Text ?? "%{vol},%{word}";
            model["model"] = GetComboBoxText(ModelBox) ?? "base.en";
            model["lang"] = LangBox.Text ?? "en";

            var outputArray = new TomlArray();
            if (StdoutCheck.IsChecked == true) outputArray.Add("stdout");
            if (TcpCheck.IsChecked == true) outputArray.Add("tcp");
            if (UdpCheck.IsChecked == true) outputArray.Add("udp");
            if (UdpBroadcastCheck.IsChecked == true) outputArray.Add("udp-broadcast");
            if (IpcCheck.IsChecked == true) outputArray.Add("ipc");
            model["output"] = outputArray;

            if (int.TryParse(PortBox.Text, out var port))
                model["port"] = port;

            model["instant"] = InstantCheck.IsChecked == true;

            model["subnet_mask"] = SubnetMaskBox.Text ?? "255.255.255.0";
            model["socket_file"] = SocketFileBox.Text ?? "./dmvop.sock";

            if (!string.IsNullOrWhiteSpace(ModelsDirBox.Text))
                model["models_dir"] = ModelsDirBox.Text;

            if (!string.IsNullOrWhiteSpace(PostBox.Text))
                model["post"] = PostBox.Text;

            var toml = Toml.FromModel(model);
            var header = """
                ## DMVOP Config File
                ## Generated by DMVOP Manager
                ##
                ## Name this file `dmvop.toml` and place it in the working dir to be auto-detected!
                ## DMVOP will auto-load this config file to avoid repeated CLI args

                """;

            File.WriteAllText(_guiConfigPath, header + toml);

            StatusLabel.Text = $"Saved: {Path.GetFileName(_guiConfigPath)}";
            ClearDirty();
        }
        catch (Exception ex)
        {
            StatusLabel.Text = $"Save error: {ex.Message}";
        }
    }

    // ---- Helpers ----

    private static string? GetString(TomlTable table, string key)
        => table.TryGetValue(key, out var val) ? val?.ToString() : null;

    private static void SelectComboBoxItem(ComboBox comboBox, string text)
    {
        for (var i = 0; i < comboBox.Items.Count; i++)
        {
            if (comboBox.Items[i] is ComboBoxItem cbi && cbi.Content?.ToString() == text)
            {
                comboBox.SelectedIndex = i;
                return;
            }
        }
    }

    private static string? GetComboBoxText(ComboBox comboBox)
        => comboBox.SelectedItem is ComboBoxItem cbi
            ? cbi.Content?.ToString()
            : null;
}
