using System;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Threading;

internal static class ElevatedTemperatureAgent
{
    private const int SampleIntervalMilliseconds = 10000;
    private const int ProbeTimeoutMilliseconds = 8000;
    private static readonly string BaseDir = AppDomain.CurrentDomain.BaseDirectory;
    private static readonly string ProbePath = Path.Combine(BaseDir, "TemperatureProbe.exe");
    private static readonly string LibreDir = Path.Combine(BaseDir, "LibreHardwareMonitor");
    private static readonly string OutputDir = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData),
        "ESP32HardwareMonitor");
    private static readonly string OutputPath = Path.Combine(OutputDir, "temperature.txt");

    [STAThread]
    private static int Main(string[] args)
    {
        if (!args.Any(arg => string.Equals(arg, "--elevated-agent", StringComparison.OrdinalIgnoreCase)))
        {
            return 2;
        }

        bool createdNew;
        using (var instance = new Mutex(true, "Local\\ESP32HardwareMonitorElevatedTemperature", out createdNew))
        {
            if (!createdNew)
            {
                return 0;
            }

            while (true)
            {
                WriteReading(ReadProbe());
                Thread.Sleep(SampleIntervalMilliseconds);
            }
        }
    }

    private static string ReadProbe()
    {
        if (!File.Exists(ProbePath) || !Directory.Exists(LibreDir))
        {
            return "CPU_TEMP=NA;GPU_TEMP=NA";
        }

        try
        {
            var startInfo = new ProcessStartInfo
            {
                FileName = ProbePath,
                Arguments = QuoteArgument(LibreDir),
                WorkingDirectory = BaseDir,
                CreateNoWindow = true,
                UseShellExecute = false,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                WindowStyle = ProcessWindowStyle.Hidden
            };

            using (var process = Process.Start(startInfo))
            {
                if (!process.WaitForExit(ProbeTimeoutMilliseconds))
                {
                    try
                    {
                        process.Kill();
                    }
                    catch
                    {
                    }
                    return "CPU_TEMP=NA;GPU_TEMP=NA";
                }

                var stdout = process.StandardOutput.ReadToEnd();
                var stderr = process.StandardError.ReadToEnd();
                if (process.ExitCode != 0 || !string.IsNullOrWhiteSpace(stderr))
                {
                    return "CPU_TEMP=NA;GPU_TEMP=NA";
                }

                return NormalizeReading(stdout);
            }
        }
        catch
        {
            return "CPU_TEMP=NA;GPU_TEMP=NA";
        }
    }

    private static string NormalizeReading(string value)
    {
        var cpu = ReadField(value, "CPU_TEMP");
        var gpu = ReadField(value, "GPU_TEMP");
        return "CPU_TEMP=" + cpu + ";GPU_TEMP=" + gpu;
    }

    private static string ReadField(string value, string name)
    {
        foreach (var part in (value ?? string.Empty).Split(';'))
        {
            var pair = part.Trim();
            if (!pair.StartsWith(name + "=", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            int parsed;
            var raw = pair.Substring(name.Length + 1).Trim();
            if (int.TryParse(raw, NumberStyles.Integer, CultureInfo.InvariantCulture, out parsed)
                && parsed > 0
                && parsed < 150)
            {
                return parsed.ToString(CultureInfo.InvariantCulture);
            }
        }

        return "NA";
    }

    private static void WriteReading(string reading)
    {
        try
        {
            Directory.CreateDirectory(OutputDir);
            var timestamp = ((long)(DateTime.UtcNow - new DateTime(1970, 1, 1)).TotalSeconds)
                .ToString(CultureInfo.InvariantCulture);
            var content = "VERSION=1;TIMESTAMP=" + timestamp + ";" + reading + Environment.NewLine;
            var temporary = OutputPath + ".new";
            File.WriteAllText(temporary, content);

            if (File.Exists(OutputPath))
            {
                File.Replace(temporary, OutputPath, null);
            }
            else
            {
                File.Move(temporary, OutputPath);
            }
        }
        catch
        {
        }
    }

    private static string QuoteArgument(string value)
    {
        return "\"" + value.Replace("\"", "\\\"") + "\"";
    }
}
