using System;
using System.IO;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using UnityEngine;
using UnityEngine.Events;

// Chinese:
// dmvop --output=tcp --port=5117 --model=small --lang=zh --instant --post="+pinyin" --device=YOUR_DEVICE
//
// English
// dmvop --output=tcp --port=5117 --model=small.en --lang=en --instant --device=YOUR_DEVICE

[Serializable]
public struct DMVOPEvent
{
    public float volume;
    public string sentence;

    public DMVOPEvent(float volume, string sentence)
    {
        this.volume = volume;
        this.sentence = sentence;
    }
}

/// <summary>
/// Connects to dmvop via TCP and receives voice input in real-time.
/// Attach to any GameObject. Wire up OnVoiceInput in the Inspector.
/// </summary>
public class DMVOPListener : MonoBehaviour
{
    [Header("DMVOP Connection")]
    public string host = "127.0.0.1";
    public int port = 5117;
    public bool connectOnStart = true;

    [Header("Events")]
    public UnityEvent<DMVOPEvent> onVoiceInput;

    private TcpClient _client;
    private Thread _readerThread;
    private volatile bool _running;
    private readonly object _lockObj = new();
    private DMVOPEvent _pendingEvent;
    private bool _hasPending;

    private void Start()
    {
        if (connectOnStart) Connect();
    }

    private void Connect()
    {
        if (_running) return;
        _running = true;

        try
        {
            _client = new TcpClient();
            _client.Connect(host, port);
            Debug.Log($"[DMVOP] Connected to {host}:{port}");

            _readerThread = new Thread(ReadLoop);
            _readerThread.IsBackground = true;
            _readerThread.Start();
        }
        catch (Exception e)
        {
            Debug.LogError($"[DMVOP] Failed to connect: {e.Message}");
            _running = false;
        }
    }

    private void Disconnect()
    {
        _running = false;
        if (_client != null)
        {
            _client.Close();
            _client = null;
        }
    }

    private void OnDestroy()
    {
        Disconnect();
    }

    private void Update()
    {
        if (!_hasPending) return;

        DMVOPEvent evt;
        lock (_lockObj)
        {
            evt = _pendingEvent;
            _hasPending = false;
        }

        Debug.Log($"[DMVOP] Received event: volume={evt.volume}, sentence='{evt.sentence}'");
        onVoiceInput?.Invoke(evt);
    }

    private void ReadLoop()
    {
        try
        {
            using var stream = _client.GetStream();
            using var reader = new StreamReader(stream, Encoding.UTF8);

            while (_running)
            {
                string line = reader.ReadLine();
                if (line == null) break;

                var evt = ParseLine(line);
                lock (_lockObj)
                {
                    _pendingEvent = evt;
                    _hasPending = true;
                }
            }
        }
        catch (Exception e) when (_running)
        {
            Debug.LogError($"[DMVOP] Connection lost: {e.Message}");
        }
        finally
        {
            _running = false;
            Debug.Log("[DMVOP] Disconnected");
        }
    }

    private static DMVOPEvent ParseLine(string line)
    {
        int comma = line.IndexOf(',');
        if (comma > 0 && float.TryParse(line.AsSpan(0, comma), out float vol))
        {
            return new DMVOPEvent(vol, line.Substring(comma + 1));
        }
        return new DMVOPEvent(0, line);
    }
}
