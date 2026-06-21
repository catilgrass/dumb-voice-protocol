using System;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;

namespace manager;

public enum MessageBoxResult
{
    Ok,
    Yes,
    No,
    Cancel,
}

public partial class MessageBox : Window
{
    public static readonly StyledProperty<string> TextProperty =
        AvaloniaProperty.Register<MessageBox, string>(nameof(Text));

    public string Text
    {
        get => GetValue(TextProperty);
        set => SetValue(TextProperty, value);
    }

    public MessageBox()
    {
        InitializeComponent();
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == TextProperty)
            MessageText.Text = change.NewValue as string ?? "";
    }

    public void AddButtons(params (string text, MessageBoxResult result)[] buttons)
    {
        foreach (var (text, result) in buttons)
        {
            var btn = new Button { Content = text, Width = 80 };
            btn.Click += (_, _) => Close(result);
            ButtonPanel.Children.Add(btn);
        }
    }

    public static Task<MessageBoxResult> Show(Window owner, string text, string title,
        params (string text, MessageBoxResult result)[] buttons)
    {
        var msgBox = new MessageBox
        {
            Title = title,
            Text = text,
        };
        msgBox.AddButtons(buttons);
        return msgBox.ShowDialog<MessageBoxResult>(owner);
    }
}
