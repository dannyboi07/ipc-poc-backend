# $pipe = New-Object System.IO.Pipes.NamedPipeClientStream(".", "as_ipc_poc_pipe", [System.IO.Pipes.PipeAccessRights]::ReadWrite)
# $pipe.Connect()
# $streamWriter = New-Object System.IO.StreamWriter($pipe)
# $streamWriter.WriteLine("Hello from PowerShell")
# $streamWriter.Flush()

$pipe = New-Object System.IO.Pipes.NamedPipeClientStream(".", "as_ipc_poc_pipe", [System.IO.Pipes.PipeDirection]::InOut)
$pipe.Connect()
$streamWriter = New-Object System.IO.StreamWriter($pipe)
# $streamWriter.WriteLine("Hello from PowerShell")
# $streamWriter.WriteLine("flip=1&thumb_size=Low&force_libraw=false&jpeg_sidecar=true&global_id=95af5d9c-c0ec-4da6-a6cf-52a55823a895&path=QzpcVXNlcnNcZGFuaWVcd29ya1xBbGJ1bVwyOTgtMjk4IER1cGxpY2F0ZVwxMiBqcGVncyAyXERTQzA0NzQ3LmpwZ")
$streamWriter.WriteLine("321:./images/12 jpegs 2/1.jpg")
$streamWriter.Flush()

$streamReader = New-Object System.IO.StreamReader($pipe)
$response = $streamReader.ReadLine()
Write-Host "Received from server: $response"

$pipe.Close()

