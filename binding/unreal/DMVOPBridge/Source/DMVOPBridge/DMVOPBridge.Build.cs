using UnrealBuildTool;

public class DMVOPBridge : ModuleRules
{
    public DMVOPBridge(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;
        PublicDependencyModuleNames.AddRange(new string[] {
            "Core", "CoreUObject", "Engine",
            "Sockets", "Networking"
        });
    }
}
