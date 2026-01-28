#!/usr/bin/env python3

import argparse
import sys
import json
import boto3
from pycognito.aws_srp import AWSSRP


def parse_args():
    parser = argparse.ArgumentParser(
        description="AWS Cognito SRP Login (pycognito CLI)"
    )

    parser.add_argument("--region", required=True, help="AWS region (e.g. ap-south-1)")
    parser.add_argument("--user-pool-id", required=True, help="Cognito User Pool ID")
    parser.add_argument("--client-id", required=True, help="Cognito App Client ID")
    parser.add_argument("--username", required=True, help="Username / phone / email")
    parser.add_argument("--password", required=True, help="User password")

    parser.add_argument(
        "--json",
        action="store_true",
        help="Output full response as JSON"
    )

    return parser.parse_args()


def main():
    args = parse_args()

    try:
        client = boto3.client(
            "cognito-idp",
            region_name=args.region
        )

        srp = AWSSRP(
            username=args.username,
            password=args.password,
            pool_id=args.user_pool_id,
            client_id=args.client_id,
            client=client
        )

        response = srp.authenticate_user()

        auth = response.get("AuthenticationResult")

        if not auth:
            print("Authentication succeeded but no tokens returned", file=sys.stderr)
            sys.exit(2)

        if args.json:
            print(json.dumps(auth, indent=2))
        else:
            print("AccessToken:\n", auth.get("AccessToken"), "\n")
            print("IdToken:\n", auth.get("IdToken"), "\n")
            print("RefreshToken:\n", auth.get("RefreshToken"))

    except Exception as e:
        print("ERROR:", str(e), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()