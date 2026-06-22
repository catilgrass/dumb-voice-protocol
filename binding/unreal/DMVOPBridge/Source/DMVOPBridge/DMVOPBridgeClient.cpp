#include "DMVOPBridgeClient.h"
#include "IPAddress.h"
#include "Interfaces/IPv4/IPv4Address.h"

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
// Undef Windows macros that conflict with UE types
#ifdef SetPort
#undef SetPort
#endif
#endif

// ═════════════════════════════════════════════════════════════════════════════
// FDMVOPWorker
// ═════════════════════════════════════════════════════════════════════════════

FDMVOPWorker::FDMVOPWorker(TWeakObjectPtr<UDMVOPClient> InOwner, FString InHost,
                           int32 InPort)
    : bRun(false), Socket(nullptr), Owner(InOwner), Host(MoveTemp(InHost)),
      Port(InPort), Thread(nullptr) {}

FDMVOPWorker::~FDMVOPWorker() {
  Stop();
  if (Thread) {
    Thread->WaitForCompletion();
    delete Thread;
    Thread = nullptr;
  }
}

void FDMVOPWorker::Start() {
  Thread = FRunnableThread::Create(
      this, *FString::Printf(TEXT("DMVOP %s:%d"), *Host, Port), 128 * 1024,
      TPri_Normal);
}

bool FDMVOPWorker::Init() {
  bRun = true;
  return true;
}

uint32 FDMVOPWorker::Run() {
  // ── Create socket ──
  Socket = ISocketSubsystem::Get(PLATFORM_SOCKETSUBSYSTEM)
               ->CreateSocket(NAME_Stream, TEXT("DMVOP"), false);
  if (!Socket) {
    return 0;
  }

  int32 RecvSize = 0, SendSize = 0;
  Socket->SetReceiveBufferSize(16384, RecvSize);
  Socket->SetSendBufferSize(16384, SendSize);

  // ── Resolve address ──
  FIPv4Address AddrIP;
  if (!FIPv4Address::Parse(Host, AddrIP)) {
    Socket->Close();
    delete Socket;
    Socket = nullptr;
    return 0;
  }

  TSharedRef<FInternetAddr> DstAddr =
      ISocketSubsystem::Get(PLATFORM_SOCKETSUBSYSTEM)->CreateInternetAddr();
  DstAddr->SetIp(AddrIP.Value);
  DstAddr->SetPort(Port);

  // ── Connect ──
  if (!Socket->Connect(*DstAddr)) {
    TWeakObjectPtr<UDMVOPClient> WeakOwner = Owner;
    AsyncTask(ENamedThreads::GameThread, [WeakOwner]() {
      if (auto *Self = WeakOwner.Get())
        UE_LOG(LogTemp, Error, TEXT("DMVOP: Failed to connect"));
    });
    Socket->Close();
    delete Socket;
    Socket = nullptr;
    return 0;
  }

  TWeakObjectPtr<UDMVOPClient> WeakOwner = Owner;
  AsyncTask(ENamedThreads::GameThread, [WeakOwner]() {
    if (auto *Self = WeakOwner.Get())
      UE_LOG(LogTemp, Log, TEXT("DMVOP: Connected"));
  });

  // ── Main loop ──
  TArray<uint8> Buf;
  Buf.SetNumUninitialized(4096);
  FString Partial;

#ifdef _WIN32
  // Raw socket for comparison (same destination, bypasses UE layer)
  SOCKET RawSock = INVALID_SOCKET;
  bool bRawOK = false;
  RawSock = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
  if (RawSock != INVALID_SOCKET) {
    struct sockaddr_in RawAddr;
    FMemory::Memzero(&RawAddr, sizeof(RawAddr));
    RawAddr.sin_family = AF_INET;
    RawAddr.sin_port = htons(Port);
    inet_pton(AF_INET, TCHAR_TO_UTF8(*Host), &RawAddr.sin_addr);
    if (connect(RawSock, (struct sockaddr *)&RawAddr, sizeof(RawAddr)) == 0) {
      bRawOK = true;
      u_long Mode = 1;
      ioctlsocket(RawSock, FIONBIO, &Mode);
      UE_LOG(LogTemp, Log, TEXT("DMVOP: Raw socket connected for comparison"));
    } else {
      closesocket(RawSock);
      RawSock = INVALID_SOCKET;
    }
  }
#endif

  while (bRun) {
    FDateTime TickStart = FDateTime::UtcNow();

    // Peek check
    Socket->SetNonBlocking(true);
    int32 DummyRead = 0;
    uint8 Dummy;
    bool bPeekOK =
        Socket->Recv(&Dummy, 1, DummyRead, ESocketReceiveFlags::Peek);
    Socket->SetNonBlocking(false);

    if (!bPeekOK) {
      break;
    }

    // Receive data
    while (bRun) {
      uint32 PendingSize = 0;
      bool bHasData = Socket->HasPendingData(PendingSize) && PendingSize > 0;

#ifdef _WIN32
      // Compare: does raw socket have data when UE socket doesn't?
      bool bRawHasData = false;
      if (bRawOK) {
        char RawBuf[1];
        int RawRead = recv(RawSock, RawBuf, 1, MSG_PEEK);
        bRawHasData = (RawRead > 0);
        if (bHasData != bRawHasData) {
          static int LogCount = 0;
          if (++LogCount <= 5) {
            UE_LOG(LogTemp, Log, TEXT("DMVOP: COMPARE UE=%d Raw=%d (Peek=%d)"),
                   bHasData, bRawHasData, RawRead);
          }
        }
      }
#endif

      if (!bHasData) {
        break;
      }

      Buf.SetNumUninitialized(FMath::Max(PendingSize, 4096u));
      int32 ReadNow = 0;
      if (!Socket->Recv(Buf.GetData(), (int32)PendingSize, ReadNow,
                        ESocketReceiveFlags::None) ||
          ReadNow <= 0) {
        break;
      }

      Partial +=
          FString(ReadNow,
                  UTF8_TO_TCHAR(reinterpret_cast<const char *>(Buf.GetData())));

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

        TWeakObjectPtr<UDMVOPClient> InnerOwner = Owner;
        AsyncTask(ENamedThreads::GameThread, [InnerOwner, Vol, Text]() {
          if (auto *Self = InnerOwner.Get())
            Self->DispatchVoiceInput(Vol, Text);
        });
      }
    }

    // Sleep
    FTimespan Elapsed = FDateTime::UtcNow() - TickStart;
    float SleepSec = 0.008f - (float)Elapsed.GetTotalSeconds();
    if (SleepSec > 0.0f) {
      FPlatformProcess::Sleep(SleepSec);
    }
  }

  // ── Cleanup ──
#ifdef _WIN32
  if (bRawOK) {
    closesocket(RawSock);
  }
#endif

  if (Socket) {
    Socket->Close();
    delete Socket;
    Socket = nullptr;
  }
  return 0;
}

void FDMVOPWorker::Stop() { bRun = false; }

// ═════════════════════════════════════════════════════════════════════════════
// UDMVOPClient
// ═════════════════════════════════════════════════════════════════════════════

UDMVOPClient::UDMVOPClient() {}

UDMVOPClient::~UDMVOPClient() {
  if (Worker.IsValid()) {
    Worker.Reset();
  }
}

void UDMVOPClient::Connect(const FString &Host, int32 Port) {
  if (Worker.IsValid()) {
    UE_LOG(LogTemp, Warning, TEXT("DMVOP: Already connected"));
    return;
  }
  UE_LOG(LogTemp, Log, TEXT("DMVOP: Connecting to %s:%d..."), *Host, Port);
  Worker = MakeShareable(new FDMVOPWorker(this, Host, Port));
  Worker->Start();
}

void UDMVOPClient::Disconnect() {
  if (Worker.IsValid()) {
    Worker.Reset();
  }
}

void UDMVOPClient::DispatchVoiceInput(float Vol, const FString &Text) {
  OnVoiceInput.Broadcast(Vol, Text);
}
