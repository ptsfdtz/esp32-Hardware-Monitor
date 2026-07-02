#include "SerialProtocol.h"

#include "Metrics.h"
#include <Arduino.h>

static String inputLine = "";

static void parseData(String data);
static int getValue(String data, String key, int oldValue);

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

// 接收格式：CPU=45;GPU=52;RAM=68
static void parseData(String data)
{
  metrics[0].targetValue = getValue(data, "CPU", metrics[0].targetValue);
  metrics[1].targetValue = getValue(data, "GPU", metrics[1].targetValue);
  metrics[2].targetValue = getValue(data, "RAM", metrics[2].targetValue);

  metrics[0].targetValue = constrain(metrics[0].targetValue, 0, 100);
  metrics[1].targetValue = constrain(metrics[1].targetValue, 0, 100);
  metrics[2].targetValue = constrain(metrics[2].targetValue, 0, 100);
}

static int getValue(String data, String key, int oldValue)
{
  String target = key + "=";
  int startIndex = data.indexOf(target);

  if (startIndex == -1)
  {
    return oldValue;
  }

  startIndex += target.length();

  int endIndex = data.indexOf(';', startIndex);
  if (endIndex == -1)
  {
    endIndex = data.length();
  }

  String valueString = data.substring(startIndex, endIndex);
  return valueString.toInt();
}
