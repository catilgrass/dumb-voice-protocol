#pragma once

#include "Async/Async.h"
#include "CoreMinimal.h"
#include "SocketSubsystem.h"
#include "Sockets.h"
#include "UObject/NoExportTypes.h"
#include "UObject/WeakObjectPtr.h"
#include <atomic>
#include <thread>

#include "DMVOPBridgeClient.generated.h"

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnDMVOPVoiceInput, float, Volume,
                                             const FString &, Sentence);

UCLASS(BlueprintType)
class DMVOPBRIDGE_API UDMVOPClient : public UObject {
  GENERATED_BODY()

public:
  UDMVOPClient();
  virtual ~UDMVOPClient();

  UFUNCTION(BlueprintCallable, Category = "DMVOP")
  void Connect(const FString &Host, int32 Port);

  UFUNCTION(BlueprintCallable, Category = "DMVOP")
  void Disconnect();

  UPROPERTY(BlueprintAssignable, Category = "DMVOP")
  FOnDMVOPVoiceInput OnVoiceInput;

private:
  FSocket *Socket;
  FString PartialLine;
  std::thread ReaderThread;
  std::atomic<bool> bRunning{false};
};
