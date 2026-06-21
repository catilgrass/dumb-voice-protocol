#include "DMVOPBridgeClient.h"
#include "Common/TcpSocketBuilder.h"
#include "IPAddress.h"

UDMVOPClient::UDMVOPClient() : Socket(nullptr) {}

UDMVOPClient::~UDMVOPClient() {
  bRunning.store(false);
  // Don't call Disconnect() here — UE GC may crash.
  // Socket will be cleaned up by the OS on process exit.
}

void UDMVOPClient::Connect(const FString &Host, int32 Port) {
  if (Socket)
    return;

  FIPv4Address IP;
  if (!FIPv4Address::Parse(Host, IP)) {
    UE_LOG(LogTemp, Error, TEXT("DMVOP: Invalid IP %s"), *Host);
    return;
  }

  TSharedRef<FInternetAddr> Addr =
      ISocketSubsystem::Get(PLATFORM_SOCKETSUBSYSTEM)->CreateInternetAddr();
  Addr->SetIp(IP.Value);
  Addr->SetPort(Port);

  Socket = FTcpSocketBuilder(TEXT("DMVOPSocket")).AsNonBlocking().Build();
  if (!Socket->Connect(*Addr)) {
    UE_LOG(LogTemp, Error, TEXT("DMVOP: Failed to connect"));
    Socket->Close();
    ISocketSubsystem::Get()->DestroySocket(Socket);
    Socket = nullptr;
    return;
  }

  UE_LOG(LogTemp, Log, TEXT("DMVOP: Connected to %s:%d"), *Host, Port);

  // Start reader thread
  bRunning.store(true);
  TWeakObjectPtr<UDMVOPClient> WeakThis(this);
  FSocket *Sock = Socket;

  ReaderThread = std::thread([WeakThis, Sock]() {
    TArray<uint8> Buf;
    Buf.SetNumUninitialized(4096);
    FString Partial;

    while (true) {
      int32 Read = 0;
      if (!Sock->Recv(Buf.GetData(), Buf.Num(), Read,
                      ESocketReceiveFlags::None) ||
          Read == 0) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
        continue;
      }

      Partial += FString(
          Read, UTF8_TO_TCHAR(reinterpret_cast<const char *>(Buf.GetData())));

      int32 Idx;
      while (Partial.FindChar('\n', Idx)) {
        FString Line = Partial.Left(Idx).TrimEnd();
        Partial = Partial.Mid(Idx + 1);
        if (Line.IsEmpty())
          continue;

        float Vol = 0.0f;
        FString Text = Line;
        int32 Comma;
        if (Line.FindChar(',', Comma)) {
          Vol = FCString::Atof(*Line.Left(Comma));
          Text = Line.Mid(Comma + 1);
        }

        AsyncTask(ENamedThreads::GameThread, [WeakThis, Vol, Text]() {
          if (auto *Self = WeakThis.Get())
            Self->OnVoiceInput.Broadcast(Vol, Text);
        });
      }
    }
  });

  ReaderThread.detach();
}

void UDMVOPClient::Disconnect() {
  bRunning.store(false);
  if (Socket) {
    Socket->Close();
    Socket = nullptr;
  }
}
