using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;

internal static class TemperatureProbe
{
    private const string DefaultLibreDir = @"C:\Users\user\Downloads\LibreHardwareMonitor";
    private static string libreDir = DefaultLibreDir;

    private static int Main(string[] args)
    {
        try
        {
            if (args.Any(arg => string.Equals(arg, "--self-test", StringComparison.OrdinalIgnoreCase)))
            {
                return RunSelfTest();
            }

            libreDir = ResolveLibreDir(args);
            var debug = args.Any(arg => string.Equals(arg, "--debug", StringComparison.OrdinalIgnoreCase));
            AppDomain.CurrentDomain.AssemblyResolve += ResolveLibreAssembly;

            var libPath = Path.Combine(libreDir, "LibreHardwareMonitorLib.dll");
            if (!File.Exists(libPath))
            {
                Console.Error.WriteLine("LibreHardwareMonitorLib.dll not found: " + libPath);
                return 2;
            }

            var lib = LoadAssemblyFile(libPath);
            var computerType = lib.GetType("LibreHardwareMonitor.Hardware.Computer", true);
            var computer = Activator.CreateInstance(computerType);

            try
            {
                SetBool(computer, "IsCpuEnabled", true);
                SetBool(computer, "IsGpuEnabled", true);
                SetBool(computer, "IsMotherboardEnabled", true);

                Invoke(computer, "Open");

                for (var i = 0; i < 3; i++)
                {
                    foreach (var hardware in AsEnumerable(Get(computer, "Hardware")))
                    {
                        UpdateHardware(hardware);
                    }

                    System.Threading.Thread.Sleep(250);
                }

                var readings = new List<TempReading>();
                var hardwareList = AsEnumerable(Get(computer, "Hardware")).ToList();

                if (debug)
                {
                    Console.Error.WriteLine("Hardware count: " + hardwareList.Count);
                    foreach (var hardware in hardwareList)
                    {
                        PrintHardwareDebug(hardware, "");
                    }
                }

                foreach (var hardware in hardwareList)
                {
                    CollectTemperatures(hardware, readings);
                }

                var cpu = PickCpuTemperature(readings);
                var gpu = PickGpuTemperature(readings);

                if (debug)
                {
                    foreach (var reading in readings)
                    {
                        Console.Error.WriteLine(
                            "{0};{1};{2};{3}",
                            reading.HardwareType,
                            reading.HardwareName,
                            reading.SensorName,
                            FormatTemp(reading.Value));
                    }
                }

                Console.WriteLine(
                    "CPU_TEMP={0};GPU_TEMP={1}",
                    FormatTemp(cpu),
                    FormatTemp(gpu));
            }
            finally
            {
                Invoke(computer, "Close");
            }

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine(DescribeException(ex));
            return 1;
        }
    }

    private static string ResolveLibreDir(string[] args)
    {
        if (args.Length > 0 && Directory.Exists(args[0]))
        {
            return args[0];
        }

        var env = Environment.GetEnvironmentVariable("LIBRE_HARDWARE_MONITOR_DIR");
        if (!string.IsNullOrWhiteSpace(env) && Directory.Exists(env))
        {
            return env;
        }

        return DefaultLibreDir;
    }

    private static Assembly ResolveLibreAssembly(object sender, ResolveEventArgs args)
    {
        var name = new AssemblyName(args.Name).Name + ".dll";
        var path = Path.Combine(libreDir, name);

        if (File.Exists(path))
        {
            return LoadAssemblyFile(path);
        }

        return null;
    }

    private static Assembly LoadAssemblyFile(string path)
    {
        return Assembly.Load(File.ReadAllBytes(path));
    }

    private static string DescribeException(Exception ex)
    {
        var reflection = ex as TargetInvocationException;
        if (reflection != null && reflection.InnerException != null)
        {
            ex = reflection.InnerException;
        }

        return ex.GetType().Name + ": " + ex.Message;
    }

    private static void UpdateHardware(object hardware)
    {
        Invoke(hardware, "Update");

        foreach (var subHardware in AsEnumerable(Get(hardware, "SubHardware")))
        {
            UpdateHardware(subHardware);
        }
    }

    private static void CollectTemperatures(object hardware, List<TempReading> readings)
    {
        var hardwareType = ToText(Get(hardware, "HardwareType"));
        var hardwareName = ToText(Get(hardware, "Name"));

        foreach (var sensor in AsEnumerable(Get(hardware, "Sensors")))
        {
            if (!string.Equals(ToText(Get(sensor, "SensorType")), "Temperature", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            var value = Get(sensor, "Value");
            if (value == null)
            {
                continue;
            }

            readings.Add(new TempReading(
                hardwareType,
                hardwareName,
                ToText(Get(sensor, "Name")),
                Convert.ToSingle(value, CultureInfo.InvariantCulture)));
        }

        foreach (var subHardware in AsEnumerable(Get(hardware, "SubHardware")))
        {
            CollectTemperatures(subHardware, readings);
        }
    }

    private static void PrintHardwareDebug(object hardware, string indent)
    {
        var hardwareType = ToText(Get(hardware, "HardwareType"));
        var hardwareName = ToText(Get(hardware, "Name"));

        if (IsGpuHardware(hardwareType, hardwareName))
        {
            Console.Error.WriteLine(
                "{0}Hardware: {1};{2};vendor={3};kind={4}",
                indent,
                hardwareType,
                hardwareName,
                DetectGpuVendor(hardwareType, hardwareName),
                DetectGpuKind(hardwareType, hardwareName));
        }
        else
        {
            Console.Error.WriteLine(
                "{0}Hardware: {1};{2}",
                indent,
                hardwareType,
                hardwareName);
        }

        foreach (var sensor in AsEnumerable(Get(hardware, "Sensors")))
        {
            Console.Error.WriteLine(
                "{0}  Sensor: {1};{2};{3}",
                indent,
                ToText(Get(sensor, "SensorType")),
                ToText(Get(sensor, "Name")),
                FormatSensorValue(Get(sensor, "Value")));
        }

        foreach (var subHardware in AsEnumerable(Get(hardware, "SubHardware")))
        {
            PrintHardwareDebug(subHardware, indent + "  ");
        }
    }

    private static IEnumerable<object> AsEnumerable(object value)
    {
        var enumerable = value as IEnumerable;
        if (enumerable == null)
        {
            yield break;
        }

        foreach (var item in enumerable)
        {
            if (item != null)
            {
                yield return item;
            }
        }
    }

    private static object Get(object target, string propertyName)
    {
        return target.GetType().GetProperty(propertyName).GetValue(target, null);
    }

    private static void SetBool(object target, string propertyName, bool value)
    {
        target.GetType().GetProperty(propertyName).SetValue(target, value, null);
    }

    private static void Invoke(object target, string methodName)
    {
        target.GetType().GetMethod(methodName).Invoke(target, null);
    }

    private static string ToText(object value)
    {
        return value == null ? string.Empty : value.ToString();
    }

    private static string FormatSensorValue(object value)
    {
        if (value == null)
        {
            return "NA";
        }

        var formattable = value as IFormattable;
        if (formattable != null)
        {
            return formattable.ToString(null, CultureInfo.InvariantCulture);
        }

        return value.ToString();
    }

    private static float? PickCpuTemperature(List<TempReading> readings)
    {
        var cpu = readings
            .Where(r => r.HardwareType.IndexOf("Cpu", StringComparison.OrdinalIgnoreCase) >= 0)
            .ToList();

        return PickByNames(cpu, "package", "tdie", "tctl", "core max")
            ?? PickMax(cpu);
    }

    private static float? PickGpuTemperature(List<TempReading> readings)
    {
        var gpu = readings
            .Where(r => IsGpuHardware(r.HardwareType, r.HardwareName))
            .ToList();

        if (gpu.Count == 0)
        {
            return null;
        }

        var candidates = gpu
            .GroupBy(r => r.HardwareType + "\n" + r.HardwareName)
            .Select(g => new GpuTemperatureCandidate(
                g.First().HardwareType,
                g.First().HardwareName,
                g.ToList()))
            .Where(c => c.Temperature.HasValue)
            .OrderByDescending(c => c.Priority)
            .ThenByDescending(c => c.Temperature.Value)
            .ToList();

        if (candidates.Count == 0)
        {
            return null;
        }

        return candidates[0].Temperature;
    }

    private static float? PickGpuTemperatureFromSensors(List<TempReading> readings)
    {
        var mainSensors = readings
            .Where(r => !IsAuxiliaryGpuTemperature(r.SensorName))
            .ToList();

        return PickByNames(mainSensors, "gpu core", "graphics", "gt", "core", "edge")
            ?? PickByNames(readings, "hot spot", "hotspot")
            ?? PickMax(mainSensors)
            ?? PickMax(readings);
    }

    private static float? PickByNames(List<TempReading> readings, params string[] names)
    {
        foreach (var name in names)
        {
            var match = readings
                .Where(r => r.SensorName.IndexOf(name, StringComparison.OrdinalIgnoreCase) >= 0)
                .OrderByDescending(r => r.Value)
                .FirstOrDefault();

            if (match != null)
            {
                return match.Value;
            }
        }

        return null;
    }

    private static float? PickMax(List<TempReading> readings)
    {
        if (readings.Count == 0)
        {
            return null;
        }

        return readings.Max(r => r.Value);
    }

    private static bool IsGpuHardware(string hardwareType, string hardwareName)
    {
        var text = Normalize(hardwareType + " " + hardwareName);

        return TextContains(hardwareType, "Gpu")
            || text.IndexOf(" nvidia ", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf(" geforce ", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf(" quadro ", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf(" rtx ", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf(" gtx ", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf(" radeon ", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf(" firepro ", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf(" iris ", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf(" uhd graphics ", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf(" arc ", StringComparison.OrdinalIgnoreCase) >= 0;
    }

    private static bool IsAuxiliaryGpuTemperature(string sensorName)
    {
        var text = Normalize(sensorName);

        return text.IndexOf("memory", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("junction", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("vram", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("vrm", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("soc", StringComparison.OrdinalIgnoreCase) >= 0;
    }

    private static int GetGpuPriority(string hardwareType, string hardwareName)
    {
        var kind = DetectGpuKind(hardwareType, hardwareName);
        var vendor = DetectGpuVendor(hardwareType, hardwareName);

        var priority = 0;
        if (kind == GpuKind.Discrete)
        {
            priority += 300;
        }
        else if (kind == GpuKind.Integrated)
        {
            priority += 200;
        }
        else
        {
            priority += 100;
        }

        if (vendor != GpuVendor.Other)
        {
            priority += 10;
        }

        return priority;
    }

    private static GpuVendor DetectGpuVendor(string hardwareType, string hardwareName)
    {
        var text = Normalize(hardwareType + " " + hardwareName);

        if (text.IndexOf("nvidia", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("geforce", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("quadro", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("rtx", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("gtx", StringComparison.OrdinalIgnoreCase) >= 0)
        {
            return GpuVendor.Nvidia;
        }

        if (text.IndexOf("gpuamd", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("amd", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("radeon", StringComparison.OrdinalIgnoreCase) >= 0)
        {
            return GpuVendor.Amd;
        }

        if (text.IndexOf("gpuintel", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("intel", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("iris", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("uhd", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("arc", StringComparison.OrdinalIgnoreCase) >= 0)
        {
            return GpuVendor.Intel;
        }

        return GpuVendor.Other;
    }

    private static GpuKind DetectGpuKind(string hardwareType, string hardwareName)
    {
        var text = Normalize(hardwareType + " " + hardwareName);
        var vendor = DetectGpuVendor(hardwareType, hardwareName);

        if (text.IndexOf("discrete", StringComparison.OrdinalIgnoreCase) >= 0)
        {
            return GpuKind.Discrete;
        }

        if (text.IndexOf("integrated", StringComparison.OrdinalIgnoreCase) >= 0
            || text.IndexOf("igpu", StringComparison.OrdinalIgnoreCase) >= 0)
        {
            return GpuKind.Integrated;
        }

        if (vendor == GpuVendor.Nvidia)
        {
            return GpuKind.Discrete;
        }

        if (vendor == GpuVendor.Intel)
        {
            if (text.IndexOf("arc", StringComparison.OrdinalIgnoreCase) >= 0)
            {
                return GpuKind.Discrete;
            }

            return GpuKind.Integrated;
        }

        if (vendor == GpuVendor.Amd)
        {
            if (text.IndexOf("radeon rx", StringComparison.OrdinalIgnoreCase) >= 0
                || text.IndexOf("radeon pro", StringComparison.OrdinalIgnoreCase) >= 0
                || text.IndexOf("firepro", StringComparison.OrdinalIgnoreCase) >= 0)
            {
                return GpuKind.Discrete;
            }

            if (text.IndexOf("radeon(tm) graphics", StringComparison.OrdinalIgnoreCase) >= 0
                || text.IndexOf("radeon graphics", StringComparison.OrdinalIgnoreCase) >= 0
                || text.IndexOf("vega", StringComparison.OrdinalIgnoreCase) >= 0
                || text.IndexOf(" 660m ", StringComparison.OrdinalIgnoreCase) >= 0
                || text.IndexOf(" 680m ", StringComparison.OrdinalIgnoreCase) >= 0
                || text.IndexOf(" 740m ", StringComparison.OrdinalIgnoreCase) >= 0
                || text.IndexOf(" 760m ", StringComparison.OrdinalIgnoreCase) >= 0
                || text.IndexOf(" 780m ", StringComparison.OrdinalIgnoreCase) >= 0
                || text.IndexOf(" 870m ", StringComparison.OrdinalIgnoreCase) >= 0
                || text.IndexOf(" 880m ", StringComparison.OrdinalIgnoreCase) >= 0
                || text.IndexOf(" 890m ", StringComparison.OrdinalIgnoreCase) >= 0)
            {
                return GpuKind.Integrated;
            }

            return GpuKind.Discrete;
        }

        return GpuKind.Unknown;
    }

    private static string Normalize(string value)
    {
        return " " + (value ?? string.Empty).ToLowerInvariant() + " ";
    }

    private static bool TextContains(string value, string needle)
    {
        return value != null && value.IndexOf(needle, StringComparison.OrdinalIgnoreCase) >= 0;
    }

    private static int RunSelfTest()
    {
        var failures = 0;

        failures += AssertTemperature(
            "discrete nvidia beats intel igpu",
            61,
            PickGpuTemperature(new List<TempReading>
            {
                new TempReading("GpuIntel", "Intel(R) UHD Graphics 730", "GPU Core", 45),
                new TempReading("GpuNvidia", "NVIDIA GeForce RTX 4070", "GPU Core", 61)
            }));

        failures += AssertTemperature(
            "discrete amd beats intel igpu",
            66,
            PickGpuTemperature(new List<TempReading>
            {
                new TempReading("GpuIntel", "Intel(R) Iris Xe Graphics", "GPU Core", 44),
                new TempReading("GpuAmd", "AMD Radeon RX 7800 XT", "GPU Core", 66)
            }));

        failures += AssertTemperature(
            "intel igpu used when no discrete gpu has temperature",
            47,
            PickGpuTemperature(new List<TempReading>
            {
                new TempReading("GpuIntel", "Intel(R) UHD Graphics 730", "GPU Core", 47)
            }));

        failures += AssertTemperature(
            "intel arc is treated as discrete",
            58,
            PickGpuTemperature(new List<TempReading>
            {
                new TempReading("GpuAmd", "AMD Radeon(TM) Graphics", "GPU Core", 43),
                new TempReading("GpuIntel", "Intel(R) Arc(TM) A770 Graphics", "GPU Core", 58)
            }));

        failures += AssertTemperature(
            "main gpu core beats memory and hot spot sensors",
            62,
            PickGpuTemperature(new List<TempReading>
            {
                new TempReading("GpuNvidia", "NVIDIA GeForce RTX 4070", "GPU Memory Junction", 90),
                new TempReading("GpuNvidia", "NVIDIA GeForce RTX 4070", "GPU Hot Spot", 78),
                new TempReading("GpuNvidia", "NVIDIA GeForce RTX 4070", "GPU Core", 62)
            }));

        failures += AssertTemperature(
            "no gpu temperature returns empty",
            null,
            PickGpuTemperature(new List<TempReading>
            {
                new TempReading("Cpu", "Intel Core", "CPU Package", 50)
            }));

        if (failures == 0)
        {
            Console.WriteLine("self-test ok");
            return 0;
        }

        Console.Error.WriteLine("self-test failed: " + failures);
        return 1;
    }

    private static int AssertTemperature(string name, int? expected, float? actual)
    {
        if (!expected.HasValue && !actual.HasValue)
        {
            return 0;
        }

        if (expected.HasValue
            && actual.HasValue
            && Math.Abs(expected.Value - actual.Value) < 0.01f)
        {
            return 0;
        }

        Console.Error.WriteLine(
            "FAIL {0}: expected={1} actual={2}",
            name,
            expected.HasValue ? expected.Value.ToString(CultureInfo.InvariantCulture) : "NA",
            FormatTemp(actual));
        return 1;
    }

    private static string FormatTemp(float? value)
    {
        if (!value.HasValue || float.IsNaN(value.Value) || float.IsInfinity(value.Value))
        {
            return "NA";
        }

        return Math.Round(value.Value).ToString(CultureInfo.InvariantCulture);
    }

    private sealed class TempReading
    {
        public TempReading(string hardwareType, string hardwareName, string sensorName, float value)
        {
            HardwareType = hardwareType;
            HardwareName = hardwareName;
            SensorName = sensorName;
            Value = value;
        }

        public string HardwareType { get; private set; }
        public string HardwareName { get; private set; }
        public string SensorName { get; private set; }
        public float Value { get; private set; }
    }

    private sealed class GpuTemperatureCandidate
    {
        public GpuTemperatureCandidate(string hardwareType, string hardwareName, List<TempReading> readings)
        {
            Temperature = PickGpuTemperatureFromSensors(readings);
            Priority = GetGpuPriority(hardwareType, hardwareName);
        }

        public float? Temperature { get; private set; }
        public int Priority { get; private set; }
    }

    private enum GpuVendor
    {
        Other,
        Intel,
        Amd,
        Nvidia
    }

    private enum GpuKind
    {
        Unknown,
        Integrated,
        Discrete
    }
}
