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
                        Console.Error.WriteLine(
                            "Hardware: {0};{1}",
                            ToText(Get(hardware, "HardwareType")),
                            ToText(Get(hardware, "Name")));
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
            .Where(r => r.HardwareType.IndexOf("Gpu", StringComparison.OrdinalIgnoreCase) >= 0)
            .ToList();

        return PickByNames(gpu, "hot spot", "hotspot", "core", "gpu")
            ?? PickMax(gpu);
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
}
