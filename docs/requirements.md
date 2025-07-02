
# Description
The goal of this package/bin is to allow instructors/teachers to
automatically assess their students' CLI (command line interface) 
programming assignments.

The CLI-grader will be able to automatically check if the students'
program passed all the tests and it will return a report with the
results. The program must be easy to use at the same time that it has
the possibility of more advanced configuration.


# High Level Complete Specification
This section gives an idea of the maximum scope that this project may
reach. Any item found here is not necessarily implemented yet and maybe
never will. 

It is interesting to have an idea of the maximum scope of a
project to modulate and guide the development process. For example, some
early decisions may be made to make it easier for a possible future expansion
or the opposite, maybe the application will never reach some future
status and that allows some kinds of optimizations in the development
process, avoiding unnecessary complexities. 

<TODO reviewing>

Also, given that a detailed and complete specification from the
beginning of a software project is not an easy thing to do, only an
informal and high level specification will be given. Also, everything
in this section related to `how` things should be done is more of a
suggestion than a rigid demand.

## Motivation
This tool will be used to **test and assess programming assignment artifacts**, 
automating part of the process. An instructor will create an **assessment
configuration artifact** and will use the this program to **grade** his
students' programs/source code. The results will be generated as a
a **report artifact**.

Examples of CLI programs that could be tested:
    - Easy (only basic unit tests with exit status code, stdout, etc):
      echo, calculators, simple CLI programs that receive input and
      return textual output.
    - Medium (more advanced unit testing, timeout/timer functionality,
      random input, more advanced output checking): timer, random number
      generator, lexer, parser. 
    - Advanced (store previous execution states, performance assessment,
      file system, network, mock, etc): http client, grep, data base
      engine.
    - Very advanced/out of scope (concurrency test, distributed
      algorithms, shared memory, etc)... 

## Generic Specifications
1. This program will be created to test CLI applications with the focus on
   learning environment, thus, **it is not a professional software testing tool**.
2. This application will be **modular** and each module should be decoupled
   from each other as much as possible. 
3. This is a **CLI tool** and building a GUI/TUI is not a priority, even
   though it would not be difficult to add graphical interfaces in the
   future, given that this application is modular and the interface is
   decoupled from the main logic. 
4. The focus will be on the CLI tool, but it will be offered as **lib**
   as well, allowing users to create their customized CLI using the 
   functionality from this crate.
5. **Performance is desirable, but not a priority**.
   - It would be interesting if we have a concurrent environment with
     independent tests running concurrently (and potentially in
     parallel).
6. **Security is highly desirable**, but given that this tool 
   will not run on any production/critical environment, only the necessary 
   for a decent application/crate will be provided, the user must be aware of
   this tool's limitations and must be responsible for any improper use.
   It would be interesting to give the option to execute the tests in a
   sandbox environment.
7. This tool must be **well tested**, specially its critical parts 
   (grading and generating reports). **Correctness** is obviously a crucial 
   requirement. Remember, someone else's grade may be defined by this tool.
8. Although implemented in Rust (🦀❤️), this tool is **agnostic in terms
   of programming languages that were used to create the CLI programs it
   grades**. 
9. This program grades CLI applications, but it does necessarily need to
   receive them already compiled. It will be able to **automate the
   compilation process of the target programs** too. But, once 
   compiled/interpreted, the behavior of the target program will be as a
   normal CLI application.
   - Main compiled languages supported: C, C++, Rust, Go, TypeScript
   - Main interpreted/bytecode languages supported: Python, JavaScript
10. The **user experience** will also be a priority.
    - This includes good error handling and diagnostics.
11. **Public API** and any obscure/hard part of the source code of this program
    **should be well documented**.

## Structure
The main modules that compose this application are:
![CLI grader diagram - Modules](./images/cli_grader_diagram.png)
- **cli**: this is the cli related part of the application, including the
  entry point (main.rs), its commands, all the structure that will
  orchestrate the application.
  - The cli will be able to execute only one test or a batch of tests at
    the same time.
- **configuration**: this module will be responsible for parsing the
  input configuration from the instructor. It may parse multiple types
  of inputs: JSON, YAML and a specific DSL. After parsing the
  configuration file, it will create a configuration data structure that
  will be used by other parts of the application. Other functions of
  this element are;
  - Check the correctness of a configuration file.
  - Generate a template for the configuration file.
  - Generate a public version of a configuration file which may be used
    by the students to run public tests on during the development of the
    assignment.
- **input**: this part is focused on implementing multiple strategies
  for compiling/interpreting the user input as source code. It generates
  a command ready to be executed in the tests. For example, if it is a
  python source code, a command like: "python program.py" will be
  provided as the strategy to be used to execute the student's CLI.
  - It will be able to receive a repository url and then clone the code
  - If no specific argument is given, this program should recognize the
    strategy to use using some reasonable heuristics: file extension,
    etc. For example: `clgrader conf.json program.py`
  - May also analyze source code to get information that will compose
    the grade.
- **grader**: this is the main core of this application. It runs
  the user's artifacts (programs and configuration files) against the 
  tests created by the professor. It will generate all the necessary
  information for the report module to use. 
  It will execute a complete test session, with target programs,
  artifacts, environment variables, name, grading configurations, and
  sections/main section. Each section may have multiple testing
  elements.
  The grading system may be absolute (100% or 0%) or not (weighted).
  The grader will allow different strategies for testing the target
  program.
  - **Unit test**: This is the most simple and fast way to create the
    test cases. It will allow **setup and tear down configurations**. Each
    test case will be ran in isolation and will be focused on only one
    program/command. It will allow easy table testing, including the possibility
    of a template like the following:
    ```json
    {
        "unit_tests":{
            "program1":[
                ["stdin", "stdout"],
                ["in1", "out1"],
                ["in2", "out2"],
                ["in3", "out3"]
            ]
        }
    }
    ```
    The unit test will be enough for a lot of use cases. Any kind of
    assignment of the type: `in -> out` which does not require complex
    shell interaction will be covered. 
    Another example would be:
    ```json
    {
        "unit_tests":{
            "program1":{
                "environment": {
                    "HOME": "/tmp/student",
                    "PATH": "/usr/bin:/bin"
                },
                {
                    "args": "\"hello  there\"", 
                    "stdout": "hello  there", 
                    "stderr": "", 
                    "status": 0, 
                    "file":{
                        "name":"out.txt",
                        "content":"some content\n..."
                    },
                    "weight":2
                    
                },
                {"args": {"permutations":["arg1", "-arg2 abc"]}, "stdout": "other example\n", "exit":{"between":[1, 5]}}
            }
        }
    }
    ```

  - **Integration test**: this kind of test allows a more detailed
    control over the test, allowing intermediate actions and checks. One
    example:
    ```json
    {
        "integrated_tests":{
            "stop_if_fail":true,
            "steps":[
                "command1 arg1 arg2 > file.txt",
                {
                    "program":"program1",
                    "args": "other example\n", 
                    "stdout": "other example\n",
                    "timeout":"5s"
                },
                "command1 arg1 arg2 > file.txt",
                {
                    "program":"program1",
                    "args": "other example\n", 
                    "stdout": {
                        "regex":"abc[a|b]+cde"
                    },
                    "weight":5
                }
            ]
        }
    }
    ```
    I believe most of the CLI programs that would be tested in a
    programming assignment class would be testable using
    `integration_test`.
  - **Performance test**: This test is only focused on benchmarking the
    performance of the input program:
    ```json
    {
        "performance_tests":{
            "program1": {
                "memory_limit": "256MB",
                "time_limit": "5s",
                "cpu_limit": "100%",
                "benchmarks": [
                    {
                        "name": "small_input",
                        "args": "input1.txt",
                        "expected_max_time": "100ms",
                        "expected_max_memory": "50MB",
                        "weight": 1
                    },
                    {
                        "name": "large_input", 
                        "args": "large_input.txt",
                        "expected_max_time": "2s",
                        "expected_max_memory": "200MB",
                        "weight": 3
                    }
                ],
                "stress_test": {
                    "enabled": true,
                    "iterations": 100,
                    "input_generator": "random_data.py",
                    "stability_threshold": 0.95
                },
                "profiling": {
                    "enabled": true,
                    "tools": ["valgrind", "perf"],
                    "memory_leaks": "fail_on_leak"
                }
            }
    

        }
    }
    ```
  Upon finishing the execution of the tests, this component will send
  the generated information for the report component.
  - During the execution, it will generate logging information.

- **report**: the report module will be responsible for the **generation
  of the final report of the student**. It will use the data from the
  grade component and it will be guided by the specifications in the
  configuration. It will allow the representation in multiple formats:
  **textual to stdout (default), txt, markdown, pdf**. Also, it will allow
  the **verbose, non-verbose, only-score** mode.
  The structure of the report will be based on the structure of the
  configuration file and the verbosity.



## Examples
This section will describe in an informal way most of the
functionalities desired for this application. 

- A programming teacher wants to create tests for his students. He
  describes the assessment configuration using a JSON file with some
  parts of the file being labeled as pub (public). The public section
  (or parts) will be available for his students to use in their
  development. The teacher does not want all his tests being available
  to his students, otherwise he thinks they could cheat easier with the
  tests available. He creates the public (incomplete) tests from his 
  (complete) private version with one single command.
- An assessment is composed of meta data and sections. Each section
  may have actions, assertions, and meta data. Actions are shell commands, which may
  include the execution of the (or some of the) program sent by the
  student. 
- An instructor wants to test a cli assignment which implements
  the `echo` functionality. He creates the following assessment
  configuration:
  ```json
  {
      "":"// It may sound a little verbose (and it is) to use JSON, but",
      "":"// it is simpler than creating an specific DSL implementation.",
      "name": "Assignment 1: Echo",
      "grading":{
          "":"// By default, grading is weighted and each assertion is worth 1 point",
          "":"// You can set weight for any section, unit test, or",
          "":"// integrated tests",
          "":"// (comments) for now, a binary grading / weighted grading is enough",
          "is_binary":true 
      },
      "report":{
          "is_verbose":false
      },

      "":"// The executable must be passed to the CLI-grader using",
      "":"// key-value or --executables -e list.",
      "executables":[
          "student_cli"
      ],
      "sections": [
          "sanity_check": {
              "is_public": true,
              "unit_tests":[
                  "student_cli":[
                      {"args": "hello  there", "stdout": "hello there"},
                      {"args": "hello there", "stdout": "hello there"}
                  ]
              ]
          },
          "complete assessment": {
              "unit_tests":[
                  "setUp":[
                    "command 1",
                    "command 2"
                  ],
                  "tearDown":[
                    "command 1"
                  ],
                  "student_cli":[
                      {"args": "\"hello  there\"", "stdout": "hello  there"},
                      {"args": "other example\n", "stdout": "other example\n"}
                  ]
              ],
              "integrated_tests":[
                  "student_cli":[
                      {"args": "\"hello  there\"", "stdout": "hello  there"},
                      {"args": "other example\n", "stdout": "other example\n"}
                  ]
              ]
          }
      ]
  }
  ```






# Requirements

This part of the document contains a more detailed and formal
specification and will be the main guide for the implementation.

## Part 1
In the end of this part, you will have a very simple, but functional
version of the application. Only a limited subset of the application
will be implemented. The focus here will be on breadth instead of deep.

1. **cli**: 
    - [ ] Create all the basic structure for the CLI: lexer/parser,
      commands structure, etc. Some libs that may be helpful: clap,
      clap-derive.
    - [ ] Command **help**: it will be called by using arguments `-h`
      or `--help` and must print to stdout the basic help message:
      ```txt
      Usage: clgrader <CONFIGURATION_FILE> <TARGET_PROGRAM>
      Grade TARGET_PROGRAM using the specification of CONFIGURATION_FILE.
      ```
    - [ ] Implement the basic usage command: `clgrader <CONFIGURATION_FILE> <TARGET_PROGRAM>`.
      In other words, this will be the complete orchestration of each
      component implemented in this stage.

2. **configuration**: 
    - [ ] Implement a parser for the configuration files which will be
      written in JSON format. The specification is:
      ```
      TODO...

      ```

