#pragma once

#include "Async/Async.h"
#include "CoreMinimal.h"
#include "HAL/Runnable.h"
#include "HAL/RunnableThread.h"
#include "HAL/ThreadSafeBool.h"
#include "SocketSubsystem.h"
#include "Sockets.h"
#include "UObject/NoExportTypes.h"
#include "UObject/WeakObjectPtr.h"
#include <atomic>

#include "DMVOPBridgeClient.generated.h"

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnDMVOPVoiceInput, float, Volume,
                                             const FString &, Sentence);

class FDMVOPWorker : public FRunnable {
public:
  FDMVOPWorker(TWeakObjectPtr<class UDMVOPClient> InOwner, FString InHost,
               int32 InPort);
  virtual ~FDMVOPWorker();

  void Start();

  // FRunnable
  virtual bool Init() override;
  virtual uint32 Run() override;
  virtual void Stop() override;

private:
  FThreadSafeBool bRun;
  FSocket *Socket;
  TWeakObjectPtr<UDMVOPClient> Owner;
  FString Host;
  int32 Port;
  FRunnableThread *Thread;
};

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

  void DispatchVoiceInput(float Vol, const FString &Text);

private:
  TSharedPtr<FDMVOPWorker> Worker;
  FString PartialLine;
};
