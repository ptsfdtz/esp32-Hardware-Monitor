#include "SerialProtocol.h"

#include "Metrics.h"
#include <Arduino.h>

static String inputLine = "";

static void parseData(const String &data);
static bool getField(const String &data, const String &key, String &value);
static int getPercentValue(const String &data, const String &key, int oldValue);
static int getTemperatureValue(const String &data, const String &key, int oldValue);

void readSerialData()
{
  while (Serial.available())
  {
    char c = Serial.read();

    if (c == '\n')
    {
      inputLine.trim();

      if (inputLine.length() > 0)
      {
        parseData(inputLine);
      }

      inputLine = "";
    }
    else
    {
      inputLine += c;
    }
  }
}

// 接收格式：CPU=45;GPU=52;RAM=68;CPU_TEMP=56;GPU_TEMP=61
static void parseData(const String &data)
{
  metrics[0].targetValue = getPercentValue(data, "CPU", metrics[0].targetValue);
  metrics[1].targetValue = getPercentValue(data, "GPU", metrics[1].targetValue);
  metrics[2].targetValue = getPercentValue(data, "RAM", metrics[2].targetValue);

  temperatures[0].value = getTemperatureValue(data, "CPU_TEMP", temperatures[0].value);
  temperatures[1].value = getTemperatureValue(data, "GPU_TEMP", temperatures[1].value);
}

static bool getField(const String &data, const String &key, String &value)
{
  String target = key + "=";
  int startIndex = data.indexOf(target);

  if (startIndex == -1)
  {
    return false;
  }

  startIndex += target.length();

  int endIndex = data.indexOf(';', startIndex);
  if (endIndex == -1)
  {
    endIndex = data.length();
  }

  value = data.substring(startIndex, endIndex);
  value.trim();
  return true;
}

static int getPercentValue(const String &data, const String &key, int oldValue)
{
  String valueString;
  if (!getField(data, key, valueString))
  {
    return oldValue;
  }

  return constrain(valueString.toInt(), 0, 100);
}

static int getTemperatureValue(const String &data, const String &key, int oldValue)
{
  String valueString;
  if (!getField(data, key, valueString))
  {
    return oldValue;
  }

  if (valueString.equalsIgnoreCase("NA") || valueString.length() == 0)
  {
    return UNKNOWN_TEMPERATURE;
  }

  return constrain(valueString.toInt(), 0, 199);
}
